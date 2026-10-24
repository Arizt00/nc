pub(crate) fn is_required(body: &str) -> bool {
    ["@malloc(", "@calloc(", "@realloc(", "@free("]
        .iter()
        .any(|needle| body.contains(needle))
}

/// Internal, OS-free allocator used by freestanding profiles. Blocks carry a
/// 16-byte header, are split on allocation and coalesced in both directions on
/// release. The allocator is synchronized and never imports libc or an OS ABI.
pub(crate) fn render() -> &'static str {
    r#"
@__ncc_alloc_heap = internal global [1048576 x i8] zeroinitializer, align 16
@__ncc_alloc_bump = internal global i64 0, align 8
@__ncc_alloc_lock = internal global i32 0, align 4

define internal void @__ncc_alloc_lock_acquire() nounwind {
entry:
  br label %attempt
attempt:
  %pair = cmpxchg ptr @__ncc_alloc_lock, i32 0, i32 1 acquire monotonic, align 4
  %locked = extractvalue { i32, i1 } %pair, 1
  br i1 %locked, label %done, label %spin
spin:
  call void asm sideeffect "", "~{memory}"()
  br label %attempt
done:
  ret void
}

define internal void @__ncc_alloc_lock_release() nounwind {
entry:
  store atomic i32 0, ptr @__ncc_alloc_lock release, align 4
  ret void
}

define internal void @__ncc_alloc_zero(ptr %data, i64 %bytes) noinline optnone nounwind {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %finish, label %body
body:
  %byte.ptr = getelementptr i8, ptr %data, i64 %i
  store i8 0, ptr %byte.ptr, align 1
  %next = add nuw i64 %i, 1
  br label %loop
finish:
  ret void
}

define internal void @__ncc_alloc_copy(ptr %destination, ptr %source, i64 %bytes) noinline optnone nounwind {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %next, %body ]
  %done = icmp uge i64 %i, %bytes
  br i1 %done, label %finish, label %body
body:
  %src = getelementptr i8, ptr %source, i64 %i
  %dst = getelementptr i8, ptr %destination, i64 %i
  %value = load i8, ptr %src, align 1
  store i8 %value, ptr %dst, align 1
  %next = add nuw i64 %i, 1
  br label %loop
finish:
  ret void
}

define ptr @malloc(i64 %requested) nounwind {
entry:
  %nonzero = icmp ne i64 %requested, 0
  %size.raw = select i1 %nonzero, i64 %requested, i64 1
  %size.valid = icmp ule i64 %size.raw, 1048544
  br i1 %size.valid, label %align, label %fail.unlocked
align:
  %size.plus = add nuw i64 %size.raw, 15
  %size = and i64 %size.plus, -16
  call void @__ncc_alloc_lock_acquire()
  %bump = load i64, ptr @__ncc_alloc_bump, align 8
  br label %scan
scan:
  %offset = phi i64 [ 0, %align ], [ %next.offset, %advance ]
  %at.end = icmp uge i64 %offset, %bump
  br i1 %at.end, label %grow, label %inspect
inspect:
  %header = getelementptr [1048576 x i8], ptr @__ncc_alloc_heap, i64 0, i64 %offset
  %state.ptr = getelementptr i8, ptr %header, i64 8
  %block.size = load i64, ptr %header, align 8
  %state = load i64, ptr %state.ptr, align 8
  %free.block = icmp eq i64 %state, 0
  %large.enough = icmp uge i64 %block.size, %size
  %reuse = and i1 %free.block, %large.enough
  br i1 %reuse, label %reuse.block, label %advance
advance:
  %stride = add i64 %block.size, 16
  %next.offset = add i64 %offset, %stride
  br label %scan
reuse.block:
  %needed.with.header = add i64 %size, 16
  %split.limit = add i64 %needed.with.header, 16
  %can.split = icmp uge i64 %block.size, %split.limit
  br i1 %can.split, label %split, label %reuse.whole
split:
  %tail.offset = add i64 %offset, %needed.with.header
  %tail.header = getelementptr [1048576 x i8], ptr @__ncc_alloc_heap, i64 0, i64 %tail.offset
  %tail.state.ptr = getelementptr i8, ptr %tail.header, i64 8
  %tail.size.pre = sub i64 %block.size, %size
  %tail.size = sub i64 %tail.size.pre, 16
  store i64 %tail.size, ptr %tail.header, align 8
  store i64 0, ptr %tail.state.ptr, align 8
  store i64 %size, ptr %header, align 8
  br label %reuse.commit
reuse.whole:
  br label %reuse.commit
reuse.commit:
  store i64 1, ptr %state.ptr, align 8
  %reuse.data = getelementptr i8, ptr %header, i64 16
  call void @__ncc_alloc_lock_release()
  ret ptr %reuse.data
grow:
  %block.bytes = add i64 %size, 16
  %new.bump = add i64 %bump, %block.bytes
  %fits = icmp ule i64 %new.bump, 1048576
  br i1 %fits, label %append, label %fail.locked
append:
  %new.header = getelementptr [1048576 x i8], ptr @__ncc_alloc_heap, i64 0, i64 %bump
  %new.state.ptr = getelementptr i8, ptr %new.header, i64 8
  store i64 %size, ptr %new.header, align 8
  store i64 1, ptr %new.state.ptr, align 8
  store i64 %new.bump, ptr @__ncc_alloc_bump, align 8
  %new.data = getelementptr i8, ptr %new.header, i64 16
  call void @__ncc_alloc_lock_release()
  ret ptr %new.data
fail.locked:
  call void @__ncc_alloc_lock_release()
  ret ptr null
fail.unlocked:
  ret ptr null
}

define void @free(ptr %data) nounwind {
entry:
  %is.null = icmp eq ptr %data, null
  br i1 %is.null, label %done, label %prepare
prepare:
  %header = getelementptr i8, ptr %data, i64 -16
  %heap.base.int = ptrtoint ptr @__ncc_alloc_heap to i64
  %header.int = ptrtoint ptr %header to i64
  %offset = sub i64 %header.int, %heap.base.int
  call void @__ncc_alloc_lock_acquire()
  %state.ptr = getelementptr i8, ptr %header, i64 8
  store i64 0, ptr %state.ptr, align 8
  %size = load i64, ptr %header, align 8
  %next.pre = add i64 %offset, 16
  %next.offset = add i64 %next.pre, %size
  %bump = load i64, ptr @__ncc_alloc_bump, align 8
  %has.next = icmp ult i64 %next.offset, %bump
  br i1 %has.next, label %merge.next.check, label %scan.previous.entry
merge.next.check:
  %next.header = getelementptr [1048576 x i8], ptr @__ncc_alloc_heap, i64 0, i64 %next.offset
  %next.state.ptr = getelementptr i8, ptr %next.header, i64 8
  %next.state = load i64, ptr %next.state.ptr, align 8
  %next.free = icmp eq i64 %next.state, 0
  br i1 %next.free, label %merge.next, label %scan.previous.entry
merge.next:
  %next.size = load i64, ptr %next.header, align 8
  %merged.pre = add i64 %size, 16
  %merged.size = add i64 %merged.pre, %next.size
  store i64 %merged.size, ptr %header, align 8
  br label %scan.previous.entry
scan.previous.entry:
  br label %scan.previous
scan.previous:
  %scan.offset = phi i64 [ 0, %scan.previous.entry ], [ %scan.next, %scan.advance ]
  %at.current = icmp uge i64 %scan.offset, %offset
  br i1 %at.current, label %release, label %scan.inspect
scan.inspect:
  %scan.header = getelementptr [1048576 x i8], ptr @__ncc_alloc_heap, i64 0, i64 %scan.offset
  %scan.size = load i64, ptr %scan.header, align 8
  %scan.stride = add i64 %scan.size, 16
  %scan.next = add i64 %scan.offset, %scan.stride
  %is.previous = icmp eq i64 %scan.next, %offset
  br i1 %is.previous, label %merge.previous.check, label %scan.advance
scan.advance:
  br label %scan.previous
merge.previous.check:
  %scan.state.ptr = getelementptr i8, ptr %scan.header, i64 8
  %scan.state = load i64, ptr %scan.state.ptr, align 8
  %previous.free = icmp eq i64 %scan.state, 0
  br i1 %previous.free, label %merge.previous, label %release
merge.previous:
  %current.size = load i64, ptr %header, align 8
  %previous.merged.pre = add i64 %scan.size, 16
  %previous.merged = add i64 %previous.merged.pre, %current.size
  store i64 %previous.merged, ptr %scan.header, align 8
  br label %release
release:
  call void @__ncc_alloc_lock_release()
  br label %done
done:
  ret void
}

define ptr @calloc(i64 %count, i64 %element.size) nounwind {
entry:
  %count.zero = icmp eq i64 %count, 0
  br i1 %count.zero, label %allocate.zero, label %multiply
multiply:
  %total = mul i64 %count, %element.size
  %roundtrip = udiv i64 %total, %count
  %overflow = icmp ne i64 %roundtrip, %element.size
  br i1 %overflow, label %fail, label %allocate
allocate.zero:
  br label %allocate
allocate:
  %bytes = phi i64 [ 0, %allocate.zero ], [ %total, %multiply ]
  %data = call ptr @malloc(i64 %bytes)
  %is.null = icmp eq ptr %data, null
  br i1 %is.null, label %done, label %zero
zero:
  call void @__ncc_alloc_zero(ptr %data, i64 %bytes)
  br label %done
done:
  ret ptr %data
fail:
  ret ptr null
}

define ptr @realloc(ptr %old, i64 %requested) nounwind {
entry:
  %old.null = icmp eq ptr %old, null
  br i1 %old.null, label %allocate, label %size.check
allocate:
  %fresh = call ptr @malloc(i64 %requested)
  ret ptr %fresh
size.check:
  %zero = icmp eq i64 %requested, 0
  br i1 %zero, label %release.zero, label %read.old
release.zero:
  call void @free(ptr %old)
  ret ptr null
read.old:
  %old.header = getelementptr i8, ptr %old, i64 -16
  %old.size = load i64, ptr %old.header, align 8
  %already.fits = icmp uge i64 %old.size, %requested
  br i1 %already.fits, label %keep, label %grow
keep:
  ret ptr %old
grow:
  %new = call ptr @malloc(i64 %requested)
  %failed = icmp eq ptr %new, null
  br i1 %failed, label %fail, label %copy
copy:
  call void @__ncc_alloc_copy(ptr %new, ptr %old, i64 %old.size)
  br label %release.old
release.old:
  call void @free(ptr %old)
  ret ptr %new
fail:
  ret ptr null
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_is_self_contained_and_has_reuse_contracts() {
        let llvm = render();
        for symbol in ["@malloc", "@calloc", "@realloc", "@free"] {
            assert!(llvm.contains(symbol));
        }
        assert!(llvm.contains("cmpxchg ptr @__ncc_alloc_lock"));
        assert!(llvm.contains("label %split"));
        assert!(llvm.contains("label %merge.next"));
        assert!(llvm.contains("label %merge.previous"));
        assert!(!llvm.contains("declare ptr @malloc"));
    }
}
