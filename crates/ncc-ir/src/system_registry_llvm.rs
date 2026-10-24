pub(crate) fn render_system_registry_runtime(
    target: &str,
    include_cpu: bool,
    include_registry: bool,
) -> Result<String, String> {
    let target = target.to_ascii_lowercase();
    let platform = if target.contains("windows") {
        if is_32_bit_target(&target) {
            WINDOWS_I386_ABI
        } else {
            WINDOWS_ABI
        }
    } else if target.contains("darwin") || target.contains("apple") {
        DARWIN_ABI
    } else if target.contains("linux") || target.contains("android") {
        LINUX_ABI
    } else {
        return Err(format!(
            "system/registry has no direct operating-system ABI for target '{target}'"
        ));
    };
    let cpu = if include_cpu {
        if target.contains("windows") {
            WINDOWS_CPU
        } else if target.contains("darwin") || target.contains("apple") {
            DARWIN_CPU
        } else {
            LINUX_CPU
        }
    } else {
        ""
    };
    let registry = if include_registry {
        if target.contains("windows") {
            WINDOWS_REGISTRY
        } else {
            POSIX_REGISTRY
        }
    } else {
        ""
    };
    Ok(format!("{platform}\n{COMMON}\n{cpu}\n{registry}"))
}

fn is_32_bit_target(target: &str) -> bool {
    target.contains("i386")
        || target.contains("i486")
        || target.contains("i586")
        || target.contains("i686")
}

const COMMON: &str = r#"
define internal i64 @ncc.system.strlen(ptr %text) {
entry:
  br label %scan
scan:
  %index = phi i64 [ 0, %entry ], [ %next, %more ]
  %slot = getelementptr i8, ptr %text, i64 %index
  %byte = load i8, ptr %slot, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %return, label %more
more:
  %next = add i64 %index, 1
  br label %scan
return:
  ret i64 %index
}

define internal ptr @ncc.system.empty() {
entry:
  %out = call ptr @__ncc_raw_aligned_alloc(i64 1, i64 16)
  %missing = icmp eq ptr %out, null
  br i1 %missing, label %done, label %terminate
terminate:
  store i8 0, ptr %out, align 1
  br label %done
done:
  ret ptr %out
}
"#;

const WINDOWS_ABI: &str = r#"
declare i32 @GetSystemTimes(ptr, ptr, ptr)
declare i32 @RegCreateKeyExA(ptr, ptr, i32, ptr, i32, i32, ptr, ptr, ptr)
declare i32 @RegOpenKeyExA(ptr, ptr, i32, i32, ptr)
declare i32 @RegSetValueExA(ptr, ptr, i32, i32, ptr, i32)
declare i32 @RegQueryValueExA(ptr, ptr, ptr, ptr, ptr, ptr)
declare i32 @RegCloseKey(ptr)

define internal i32 @ncc.system.get.times(ptr %idle, ptr %kernel, ptr %user) {
  %status = call i32 @GetSystemTimes(ptr %idle, ptr %kernel, ptr %user)
  ret i32 %status
}
define internal i32 @ncc.system.reg.create(ptr %root, ptr %subkey, ptr %key, ptr %disposition) {
  %status = call i32 @RegCreateKeyExA(ptr %root, ptr %subkey, i32 0, ptr null, i32 0, i32 3, ptr null, ptr %key, ptr %disposition)
  ret i32 %status
}
define internal i32 @ncc.system.reg.open(ptr %root, ptr %subkey, ptr %key) {
  %status = call i32 @RegOpenKeyExA(ptr %root, ptr %subkey, i32 0, i32 1, ptr %key)
  ret i32 %status
}
define internal i32 @ncc.system.reg.set(ptr %key, ptr %name, ptr %value, i32 %bytes) {
  %status = call i32 @RegSetValueExA(ptr %key, ptr %name, i32 0, i32 1, ptr %value, i32 %bytes)
  ret i32 %status
}
define internal i32 @ncc.system.reg.query(ptr %key, ptr %name, ptr %type, ptr %value, ptr %bytes) {
  %status = call i32 @RegQueryValueExA(ptr %key, ptr %name, ptr null, ptr %type, ptr %value, ptr %bytes)
  ret i32 %status
}
define internal i32 @ncc.system.reg.close(ptr %key) {
  %status = call i32 @RegCloseKey(ptr %key)
  ret i32 %status
}
"#;

const WINDOWS_I386_ABI: &str = r#"
declare x86_stdcallcc i32 @"\01_GetSystemTimes@12"(ptr, ptr, ptr)
declare x86_stdcallcc i32 @"\01_RegCreateKeyExA@36"(ptr, ptr, i32, ptr, i32, i32, ptr, ptr, ptr)
declare x86_stdcallcc i32 @"\01_RegOpenKeyExA@20"(ptr, ptr, i32, i32, ptr)
declare x86_stdcallcc i32 @"\01_RegSetValueExA@24"(ptr, ptr, i32, i32, ptr, i32)
declare x86_stdcallcc i32 @"\01_RegQueryValueExA@24"(ptr, ptr, ptr, ptr, ptr, ptr)
declare x86_stdcallcc i32 @"\01_RegCloseKey@4"(ptr)

define internal i32 @ncc.system.get.times(ptr %idle, ptr %kernel, ptr %user) {
  %status = call x86_stdcallcc i32 @"\01_GetSystemTimes@12"(ptr %idle, ptr %kernel, ptr %user)
  ret i32 %status
}
define internal i32 @ncc.system.reg.create(ptr %root, ptr %subkey, ptr %key, ptr %disposition) {
  %status = call x86_stdcallcc i32 @"\01_RegCreateKeyExA@36"(ptr %root, ptr %subkey, i32 0, ptr null, i32 0, i32 3, ptr null, ptr %key, ptr %disposition)
  ret i32 %status
}
define internal i32 @ncc.system.reg.open(ptr %root, ptr %subkey, ptr %key) {
  %status = call x86_stdcallcc i32 @"\01_RegOpenKeyExA@20"(ptr %root, ptr %subkey, i32 0, i32 1, ptr %key)
  ret i32 %status
}
define internal i32 @ncc.system.reg.set(ptr %key, ptr %name, ptr %value, i32 %bytes) {
  %status = call x86_stdcallcc i32 @"\01_RegSetValueExA@24"(ptr %key, ptr %name, i32 0, i32 1, ptr %value, i32 %bytes)
  ret i32 %status
}
define internal i32 @ncc.system.reg.query(ptr %key, ptr %name, ptr %type, ptr %value, ptr %bytes) {
  %status = call x86_stdcallcc i32 @"\01_RegQueryValueExA@24"(ptr %key, ptr %name, ptr null, ptr %type, ptr %value, ptr %bytes)
  ret i32 %status
}
define internal i32 @ncc.system.reg.close(ptr %key) {
  %status = call x86_stdcallcc i32 @"\01_RegCloseKey@4"(ptr %key)
  ret i32 %status
}
"#;

const WINDOWS_CPU: &str = r#"
define double @ncc_rt_system_cpu_load() {
entry:
  %idle.before = alloca i64, align 8
  %kernel.before = alloca i64, align 8
  %user.before = alloca i64, align 8
  %idle.after = alloca i64, align 8
  %kernel.after = alloca i64, align 8
  %user.after = alloca i64, align 8
  %before.ok = call i32 @ncc.system.get.times(ptr %idle.before, ptr %kernel.before, ptr %user.before)
  %before.failed = icmp eq i32 %before.ok, 0
  br i1 %before.failed, label %failure, label %sample.after
sample.after:
  call void @ncc_rt_sleep_millis(i64 25)
  %after.ok = call i32 @ncc.system.get.times(ptr %idle.after, ptr %kernel.after, ptr %user.after)
  %after.failed = icmp eq i32 %after.ok, 0
  br i1 %after.failed, label %failure, label %calculate
calculate:
  %idle.0 = load i64, ptr %idle.before, align 8
  %idle.1 = load i64, ptr %idle.after, align 8
  %kernel.0 = load i64, ptr %kernel.before, align 8
  %kernel.1 = load i64, ptr %kernel.after, align 8
  %user.0 = load i64, ptr %user.before, align 8
  %user.1 = load i64, ptr %user.after, align 8
  %idle.delta = sub i64 %idle.1, %idle.0
  %kernel.delta = sub i64 %kernel.1, %kernel.0
  %user.delta = sub i64 %user.1, %user.0
  %total.delta = add i64 %kernel.delta, %user.delta
  %zero = icmp eq i64 %total.delta, 0
  br i1 %zero, label %zero.result, label %percentage
percentage:
  %idle.large = icmp ugt i64 %idle.delta, %total.delta
  %idle.clamped = select i1 %idle.large, i64 %total.delta, i64 %idle.delta
  %busy = sub i64 %total.delta, %idle.clamped
  %busy.f = uitofp i64 %busy to double
  %total.f = uitofp i64 %total.delta to double
  %scaled = fmul double %busy.f, 1.000000e+02
  %result = fdiv double %scaled, %total.f
  ret double %result
zero.result:
  ret double 0.000000e+00
failure:
  ret double -1.000000e+00
}
"#;

const WINDOWS_REGISTRY: &str = r#"
@ncc.system.registry.subkey = private unnamed_addr constant [20 x i8] c"Software\5CnC\5CRuntime\00", align 1

define i32 @ncc_rt_registry_write(ptr %name, ptr %value) {
entry:
  %key.slot = alloca ptr, align 8
  %disposition = alloca i32, align 4
  store ptr null, ptr %key.slot, align 8
  %status = call i32 @ncc.system.reg.create(ptr inttoptr (i64 2147483649 to ptr), ptr @ncc.system.registry.subkey, ptr %key.slot, ptr %disposition)
  %key = load ptr, ptr %key.slot, align 8
  %opened = icmp eq i32 %status, 0
  %has.key = icmp ne ptr %key, null
  %ready = and i1 %opened, %has.key
  br i1 %ready, label %write, label %failure
write:
  %length = call i64 @ncc.system.strlen(ptr %value)
  %bytes64 = add i64 %length, 1
  %bytes = trunc i64 %bytes64 to i32
  %set.status = call i32 @ncc.system.reg.set(ptr %key, ptr %name, ptr %value, i32 %bytes)
  %closed = call i32 @ncc.system.reg.close(ptr %key)
  %ok = icmp eq i32 %set.status, 0
  %result = zext i1 %ok to i32
  ret i32 %result
failure:
  ret i32 0
}

define ptr @ncc_rt_registry_read(ptr %name) {
entry:
  %key.slot = alloca ptr, align 8
  %type.slot = alloca i32, align 4
  %bytes.slot = alloca i32, align 4
  store ptr null, ptr %key.slot, align 8
  store i32 0, ptr %type.slot, align 4
  store i32 0, ptr %bytes.slot, align 4
  %status = call i32 @ncc.system.reg.open(ptr inttoptr (i64 2147483649 to ptr), ptr @ncc.system.registry.subkey, ptr %key.slot)
  %key = load ptr, ptr %key.slot, align 8
  %opened = icmp eq i32 %status, 0
  %has.key = icmp ne ptr %key, null
  %ready = and i1 %opened, %has.key
  br i1 %ready, label %measure, label %empty
measure:
  %measure.status = call i32 @ncc.system.reg.query(ptr %key, ptr %name, ptr %type.slot, ptr null, ptr %bytes.slot)
  %type = load i32, ptr %type.slot, align 4
  %bytes32 = load i32, ptr %bytes.slot, align 4
  %measure.ok = icmp eq i32 %measure.status, 0
  %type.ok = icmp eq i32 %type, 1
  %bytes.ok = icmp ugt i32 %bytes32, 0
  %valid.0 = and i1 %measure.ok, %type.ok
  %valid = and i1 %valid.0, %bytes.ok
  br i1 %valid, label %allocate, label %close.empty
allocate:
  %bytes64 = zext i32 %bytes32 to i64
  %capacity = add i64 %bytes64, 1
  %out = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 16)
  %missing = icmp eq ptr %out, null
  br i1 %missing, label %close.empty, label %query
query:
  %query.status = call i32 @ncc.system.reg.query(ptr %key, ptr %name, ptr %type.slot, ptr %out, ptr %bytes.slot)
  %closed = call i32 @ncc.system.reg.close(ptr %key)
  %query.type = load i32, ptr %type.slot, align 4
  %query.bytes32 = load i32, ptr %bytes.slot, align 4
  %query.ok = icmp eq i32 %query.status, 0
  %query.type.ok = icmp eq i32 %query.type, 1
  %query.valid = and i1 %query.ok, %query.type.ok
  br i1 %query.valid, label %terminate, label %release.empty
terminate:
  %query.bytes = zext i32 %query.bytes32 to i64
  %end = getelementptr i8, ptr %out, i64 %query.bytes
  store i8 0, ptr %end, align 1
  ret ptr %out
release.empty:
  %released = call i1 @__ncc_raw_aligned_free(ptr %out)
  br label %empty
close.empty:
  %closed.empty = call i32 @ncc.system.reg.close(ptr %key)
  br label %empty
empty:
  %fallback = call ptr @ncc.system.empty()
  ret ptr %fallback
}
"#;

const LINUX_ABI: &str = r#""#;

const DARWIN_ABI: &str = r#"
declare i32 @mach_host_self()
declare i32 @host_statistics(i32, i32, ptr, ptr)
"#;

const LINUX_CPU: &str = r#"
@ncc.system.proc.stat = private unnamed_addr constant [11 x i8] c"/proc/stat\00", align 1

define internal i32 @ncc.system.cpu.sample(ptr %sample) {
entry:
  %storage = alloca [512 x i8], align 16
  %buffer = getelementptr [512 x i8], ptr %storage, i32 0, i32 0
  %count = call i64 @__ncc_fs_read(ptr @ncc.system.proc.stat, ptr %buffer, i64 511)
  %has.bytes = icmp sgt i64 %count, 3
  br i1 %has.bytes, label %header, label %failure
header:
  %b0 = load i8, ptr %buffer, align 1
  %p1 = getelementptr i8, ptr %buffer, i64 1
  %b1 = load i8, ptr %p1, align 1
  %p2 = getelementptr i8, ptr %buffer, i64 2
  %b2 = load i8, ptr %p2, align 1
  %c0 = icmp eq i8 %b0, 99
  %c1 = icmp eq i8 %b1, 112
  %c2 = icmp eq i8 %b2, 117
  %h0 = and i1 %c0, %c1
  %valid.header = and i1 %h0, %c2
  br i1 %valid.header, label %parse, label %failure
parse:
  br label %skip
skip:
  %cursor = phi i64 [ 3, %parse ], [ %cursor.next, %skip.more ]
  %field = phi i32 [ 0, %parse ], [ %field, %skip.more ]
  %total = phi i64 [ 0, %parse ], [ %total, %skip.more ]
  %idle = phi i64 [ 0, %parse ], [ %idle, %skip.more ]
  %inside = icmp ult i64 %cursor, %count
  br i1 %inside, label %skip.load, label %finish
skip.load:
  %char.ptr = getelementptr i8, ptr %buffer, i64 %cursor
  %char = load i8, ptr %char.ptr, align 1
  %newline = icmp eq i8 %char, 10
  br i1 %newline, label %finish, label %skip.check
skip.check:
  %below = icmp ult i8 %char, 48
  %above = icmp ugt i8 %char, 57
  %not.digit = or i1 %below, %above
  br i1 %not.digit, label %skip.more, label %digits
skip.more:
  %cursor.next = add i64 %cursor, 1
  br label %skip
digits:
  br label %digit.loop
digit.loop:
  %digit.cursor = phi i64 [ %cursor, %digits ], [ %digit.next, %digit.more ]
  %value = phi i64 [ 0, %digits ], [ %value.next, %digit.more ]
  %digit.inside = icmp ult i64 %digit.cursor, %count
  br i1 %digit.inside, label %digit.load, label %accept
digit.load:
  %digit.ptr = getelementptr i8, ptr %buffer, i64 %digit.cursor
  %digit.char = load i8, ptr %digit.ptr, align 1
  %digit.below = icmp ult i8 %digit.char, 48
  %digit.above = icmp ugt i8 %digit.char, 57
  %digit.done = or i1 %digit.below, %digit.above
  br i1 %digit.done, label %accept, label %digit.more
digit.more:
  %digit.raw = sub i8 %digit.char, 48
  %digit.i64 = zext i8 %digit.raw to i64
  %value.scaled = mul i64 %value, 10
  %value.next = add i64 %value.scaled, %digit.i64
  %digit.next = add i64 %digit.cursor, 1
  br label %digit.loop
accept:
  %accepted.cursor = phi i64 [ %digit.cursor, %digit.loop ], [ %digit.cursor, %digit.load ]
  %accepted.value = phi i64 [ %value, %digit.loop ], [ %value, %digit.load ]
  %total.next = add i64 %total, %accepted.value
  %is.idle = icmp eq i32 %field, 3
  %is.iowait = icmp eq i32 %field, 4
  %idle.field = or i1 %is.idle, %is.iowait
  %idle.next = select i1 %idle.field, i64 %accepted.value, i64 0
  %idle.sum = add i64 %idle, %idle.next
  %field.next = add i32 %field, 1
  %enough = icmp uge i32 %field.next, 10
  br i1 %enough, label %finish.values, label %continue.fields
continue.fields:
  %continue.cursor = add i64 %accepted.cursor, 1
  br label %skip.values
skip.values:
  %cursor.values = phi i64 [ %continue.cursor, %continue.fields ], [ %cursor.values.next, %skip.values.more ]
  %inside.values = icmp ult i64 %cursor.values, %count
  br i1 %inside.values, label %load.values, label %finish.values
load.values:
  %value.ptr = getelementptr i8, ptr %buffer, i64 %cursor.values
  %value.char = load i8, ptr %value.ptr, align 1
  %value.newline = icmp eq i8 %value.char, 10
  %value.digit.low = icmp uge i8 %value.char, 48
  %value.digit.high = icmp ule i8 %value.char, 57
  %value.digit = and i1 %value.digit.low, %value.digit.high
  br i1 %value.newline, label %finish.values, label %value.choice
value.choice:
  br i1 %value.digit, label %reenter, label %skip.values.more
skip.values.more:
  %cursor.values.next = add i64 %cursor.values, 1
  br label %skip.values
reenter:
  br label %digit.values
digit.values:
  %dv.cursor = phi i64 [ %cursor.values, %reenter ], [ %dv.next, %dv.more ]
  %dv.value = phi i64 [ 0, %reenter ], [ %dv.value.next, %dv.more ]
  %dv.inside = icmp ult i64 %dv.cursor, %count
  br i1 %dv.inside, label %dv.load, label %accept.values
dv.load:
  %dv.ptr = getelementptr i8, ptr %buffer, i64 %dv.cursor
  %dv.char = load i8, ptr %dv.ptr, align 1
  %dv.low = icmp ult i8 %dv.char, 48
  %dv.high = icmp ugt i8 %dv.char, 57
  %dv.done = or i1 %dv.low, %dv.high
  br i1 %dv.done, label %accept.values, label %dv.more
dv.more:
  %dv.raw = sub i8 %dv.char, 48
  %dv.i64 = zext i8 %dv.raw to i64
  %dv.scaled = mul i64 %dv.value, 10
  %dv.value.next = add i64 %dv.scaled, %dv.i64
  %dv.next = add i64 %dv.cursor, 1
  br label %digit.values
accept.values:
  %av.cursor = phi i64 [ %dv.cursor, %digit.values ], [ %dv.cursor, %dv.load ]
  %av.value = phi i64 [ %dv.value, %digit.values ], [ %dv.value, %dv.load ]
  %av.total = add i64 %total.next, %av.value
  %av.is.idle = icmp eq i32 %field.next, 3
  %av.is.iowait = icmp eq i32 %field.next, 4
  %av.idle.field = or i1 %av.is.idle, %av.is.iowait
  %av.idle.value = select i1 %av.idle.field, i64 %av.value, i64 0
  %av.idle = add i64 %idle.sum, %av.idle.value
  %av.field = add i32 %field.next, 1
  %av.enough = icmp uge i32 %av.field, 10
  br i1 %av.enough, label %finish.values2, label %continue.values
continue.values:
  %av.next.cursor = add i64 %av.cursor, 1
  br label %skip.values2
skip.values2:
  %sv.cursor = phi i64 [ %av.next.cursor, %continue.values ], [ %sv.next, %sv.more ]
  %sv.inside = icmp ult i64 %sv.cursor, %count
  br i1 %sv.inside, label %sv.load, label %finish.values2
sv.load:
  %sv.ptr = getelementptr i8, ptr %buffer, i64 %sv.cursor
  %sv.char = load i8, ptr %sv.ptr, align 1
  %sv.newline = icmp eq i8 %sv.char, 10
  %sv.digit.low = icmp uge i8 %sv.char, 48
  %sv.digit.high = icmp ule i8 %sv.char, 57
  %sv.digit = and i1 %sv.digit.low, %sv.digit.high
  br i1 %sv.newline, label %finish.values2, label %sv.choice
sv.choice:
  br i1 %sv.digit, label %unsupported.more.fields, label %sv.more
sv.more:
  %sv.next = add i64 %sv.cursor, 1
  br label %skip.values2
unsupported.more.fields:
  br label %finish.values2
finish.values2:
  %final.total.2 = phi i64 [ %av.total, %accept.values ], [ %av.total, %skip.values2 ], [ %av.total, %sv.load ], [ %av.total, %unsupported.more.fields ]
  %final.idle.2 = phi i64 [ %av.idle, %accept.values ], [ %av.idle, %skip.values2 ], [ %av.idle, %sv.load ], [ %av.idle, %unsupported.more.fields ]
  br label %store
finish.values:
  %final.total.1 = phi i64 [ %total.next, %accept ], [ %total.next, %skip.values ], [ %total.next, %load.values ]
  %final.idle.1 = phi i64 [ %idle.sum, %accept ], [ %idle.sum, %skip.values ], [ %idle.sum, %load.values ]
  br label %store.one
finish:
  br label %failure
store.one:
  %total.slot.1 = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 0
  %idle.slot.1 = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 1
  store i64 %final.total.1, ptr %total.slot.1, align 8
  store i64 %final.idle.1, ptr %idle.slot.1, align 8
  ret i32 1
store:
  %total.slot = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 0
  %idle.slot = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 1
  store i64 %final.total.2, ptr %total.slot, align 8
  store i64 %final.idle.2, ptr %idle.slot, align 8
  ret i32 1
failure:
  ret i32 0
}

define double @ncc_rt_system_cpu_load() {
entry:
  %before = alloca { i64, i64 }, align 8
  %after = alloca { i64, i64 }, align 8
  %before.ok = call i32 @ncc.system.cpu.sample(ptr %before)
  %before.failed = icmp eq i32 %before.ok, 0
  br i1 %before.failed, label %failure, label %after.sample
after.sample:
  call void @ncc_rt_sleep_millis(i64 25)
  %after.ok = call i32 @ncc.system.cpu.sample(ptr %after)
  %after.failed = icmp eq i32 %after.ok, 0
  br i1 %after.failed, label %failure, label %calculate
calculate:
  %before.total.slot = getelementptr { i64, i64 }, ptr %before, i32 0, i32 0
  %before.idle.slot = getelementptr { i64, i64 }, ptr %before, i32 0, i32 1
  %after.total.slot = getelementptr { i64, i64 }, ptr %after, i32 0, i32 0
  %after.idle.slot = getelementptr { i64, i64 }, ptr %after, i32 0, i32 1
  %before.total = load i64, ptr %before.total.slot, align 8
  %before.idle = load i64, ptr %before.idle.slot, align 8
  %after.total = load i64, ptr %after.total.slot, align 8
  %after.idle = load i64, ptr %after.idle.slot, align 8
  %total = sub i64 %after.total, %before.total
  %idle = sub i64 %after.idle, %before.idle
  %zero = icmp eq i64 %total, 0
  br i1 %zero, label %zero.result, label %percentage
percentage:
  %idle.large = icmp ugt i64 %idle, %total
  %idle.clamped = select i1 %idle.large, i64 %total, i64 %idle
  %busy = sub i64 %total, %idle.clamped
  %busy.f = uitofp i64 %busy to double
  %total.f = uitofp i64 %total to double
  %scaled = fmul double %busy.f, 1.000000e+02
  %result = fdiv double %scaled, %total.f
  ret double %result
zero.result:
  ret double 0.000000e+00
failure:
  ret double -1.000000e+00
}
"#;

const DARWIN_CPU: &str = r#"
define internal i32 @ncc.system.cpu.sample(ptr %sample) {
entry:
  %ticks = alloca [4 x i32], align 4
  %count = alloca i32, align 4
  store i32 4, ptr %count, align 4
  %host = call i32 @mach_host_self()
  %status = call i32 @host_statistics(i32 %host, i32 3, ptr %ticks, ptr %count)
  %actual = load i32, ptr %count, align 4
  %status.ok = icmp eq i32 %status, 0
  %count.ok = icmp uge i32 %actual, 4
  %ok = and i1 %status.ok, %count.ok
  br i1 %ok, label %calculate, label %failure
calculate:
  %p0 = getelementptr [4 x i32], ptr %ticks, i32 0, i32 0
  %p1 = getelementptr [4 x i32], ptr %ticks, i32 0, i32 1
  %p2 = getelementptr [4 x i32], ptr %ticks, i32 0, i32 2
  %p3 = getelementptr [4 x i32], ptr %ticks, i32 0, i32 3
  %v0.32 = load i32, ptr %p0, align 4
  %v1.32 = load i32, ptr %p1, align 4
  %v2.32 = load i32, ptr %p2, align 4
  %v3.32 = load i32, ptr %p3, align 4
  %v0 = zext i32 %v0.32 to i64
  %v1 = zext i32 %v1.32 to i64
  %v2 = zext i32 %v2.32 to i64
  %v3 = zext i32 %v3.32 to i64
  %sum0 = add i64 %v0, %v1
  %sum1 = add i64 %v2, %v3
  %total = add i64 %sum0, %sum1
  %total.slot = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 0
  %idle.slot = getelementptr { i64, i64 }, ptr %sample, i32 0, i32 1
  store i64 %total, ptr %total.slot, align 8
  store i64 %v2, ptr %idle.slot, align 8
  ret i32 1
failure:
  ret i32 0
}
"#;

const POSIX_REGISTRY: &str = r#"
@ncc.system.home = private unnamed_addr constant [5 x i8] c"HOME\00", align 1
@ncc.system.tmpdir = private unnamed_addr constant [7 x i8] c"TMPDIR\00", align 1
@ncc.system.tmp = private unnamed_addr constant [5 x i8] c"/tmp\00", align 1
@ncc.system.registry.prefix = private unnamed_addr constant [15 x i8] c"/.nc_registry_\00", align 1
@ncc.system.hex = private unnamed_addr constant [17 x i8] c"0123456789abcdef\00", align 1

declare ptr @getenv(ptr)

define internal i64 @ncc.system.registry.hash(ptr %key) {
entry:
  br label %loop
loop:
  %cursor = phi ptr [ %key, %entry ], [ %next, %more ]
  %hash = phi i64 [ 1469598103934665603, %entry ], [ %mixed, %more ]
  %byte = load i8, ptr %cursor, align 1
  %done = icmp eq i8 %byte, 0
  br i1 %done, label %return, label %more
more:
  %unsigned = zext i8 %byte to i64
  %xored = xor i64 %hash, %unsigned
  %mixed = mul i64 %xored, 1099511628211
  %next = getelementptr i8, ptr %cursor, i64 1
  br label %loop
return:
  ret i64 %hash
}

define internal ptr @ncc.system.registry.path(ptr %key) {
entry:
  %home.initial = call ptr @getenv(ptr @ncc.system.home)
  %home.missing = icmp eq ptr %home.initial, null
  br i1 %home.missing, label %try.tmpdir, label %check.home
check.home:
  %home.first = load i8, ptr %home.initial, align 1
  %home.empty = icmp eq i8 %home.first, 0
  br i1 %home.empty, label %try.tmpdir, label %selected
try.tmpdir:
  %tmpdir = call ptr @getenv(ptr @ncc.system.tmpdir)
  %tmpdir.missing = icmp eq ptr %tmpdir, null
  br i1 %tmpdir.missing, label %fallback, label %check.tmpdir
check.tmpdir:
  %tmpdir.first = load i8, ptr %tmpdir, align 1
  %tmpdir.empty = icmp eq i8 %tmpdir.first, 0
  br i1 %tmpdir.empty, label %fallback, label %selected.tmpdir
fallback:
  br label %selected.fallback
selected.tmpdir:
  br label %selected
selected.fallback:
  br label %selected
selected:
  %home = phi ptr [ %home.initial, %check.home ], [ %tmpdir, %selected.tmpdir ], [ @ncc.system.tmp, %selected.fallback ]
  %home.length = call i64 @ncc.system.strlen(ptr %home)
  %bytes = add i64 %home.length, 31
  %path = call ptr @__ncc_raw_aligned_alloc(i64 %bytes, i64 16)
  %missing = icmp eq ptr %path, null
  br i1 %missing, label %failure, label %copy.home
copy.home:
  br label %home.loop
home.loop:
  %hi = phi i64 [ 0, %copy.home ], [ %hi.next, %home.more ]
  %home.done = icmp eq i64 %hi, %home.length
  br i1 %home.done, label %prefix.loop, label %home.more
home.more:
  %home.src = getelementptr i8, ptr %home, i64 %hi
  %home.byte = load i8, ptr %home.src, align 1
  %home.dst = getelementptr i8, ptr %path, i64 %hi
  store i8 %home.byte, ptr %home.dst, align 1
  %hi.next = add i64 %hi, 1
  br label %home.loop
prefix.loop:
  %pi = phi i64 [ 0, %home.loop ], [ %pi.next, %prefix.more ]
  %prefix.done = icmp eq i64 %pi, 14
  br i1 %prefix.done, label %hex.loop, label %prefix.more
prefix.more:
  %prefix.src = getelementptr [15 x i8], ptr @ncc.system.registry.prefix, i32 0, i64 %pi
  %prefix.byte = load i8, ptr %prefix.src, align 1
  %prefix.offset = add i64 %home.length, %pi
  %prefix.dst = getelementptr i8, ptr %path, i64 %prefix.offset
  store i8 %prefix.byte, ptr %prefix.dst, align 1
  %pi.next = add i64 %pi, 1
  br label %prefix.loop
hex.loop:
  %hash = call i64 @ncc.system.registry.hash(ptr %key)
  br label %hex.write
hex.write:
  %xi = phi i64 [ 0, %hex.loop ], [ %xi.next, %hex.more ]
  %hex.done = icmp eq i64 %xi, 16
  br i1 %hex.done, label %terminate, label %hex.more
hex.more:
  %reverse = sub i64 15, %xi
  %shift = mul i64 %reverse, 4
  %shifted = lshr i64 %hash, %shift
  %nibble = and i64 %shifted, 15
  %hex.src = getelementptr [17 x i8], ptr @ncc.system.hex, i32 0, i64 %nibble
  %hex.byte = load i8, ptr %hex.src, align 1
  %hex.base = add i64 %home.length, 14
  %hex.offset = add i64 %hex.base, %xi
  %hex.dst = getelementptr i8, ptr %path, i64 %hex.offset
  store i8 %hex.byte, ptr %hex.dst, align 1
  %xi.next = add i64 %xi, 1
  br label %hex.write
terminate:
  %end.offset = add i64 %home.length, 30
  %end = getelementptr i8, ptr %path, i64 %end.offset
  store i8 0, ptr %end, align 1
  ret ptr %path
failure:
  ret ptr null
}

define i32 @ncc_rt_registry_write(ptr %key, ptr %value) {
entry:
  %path = call ptr @ncc.system.registry.path(ptr %key)
  %path.missing = icmp eq ptr %path, null
  br i1 %path.missing, label %failure, label %allocate
allocate:
  %key.length = call i64 @ncc.system.strlen(ptr %key)
  %value.length = call i64 @ncc.system.strlen(ptr %value)
  %sum = add i64 %key.length, %value.length
  %capacity = add i64 %sum, 2
  %payload = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 16)
  %payload.missing = icmp eq ptr %payload, null
  br i1 %payload.missing, label %release.path.failure, label %copy.key
copy.key:
  br label %key.loop
key.loop:
  %ki = phi i64 [ 0, %copy.key ], [ %ki.next, %key.more ]
  %key.done = icmp eq i64 %ki, %key.length
  br i1 %key.done, label %newline, label %key.more
key.more:
  %key.src = getelementptr i8, ptr %key, i64 %ki
  %key.byte = load i8, ptr %key.src, align 1
  %key.dst = getelementptr i8, ptr %payload, i64 %ki
  store i8 %key.byte, ptr %key.dst, align 1
  %ki.next = add i64 %ki, 1
  br label %key.loop
newline:
  %newline.slot = getelementptr i8, ptr %payload, i64 %key.length
  store i8 10, ptr %newline.slot, align 1
  br label %value.loop
value.loop:
  %vi = phi i64 [ 0, %newline ], [ %vi.next, %value.more ]
  %value.done = icmp eq i64 %vi, %value.length
  br i1 %value.done, label %terminate, label %value.more
value.more:
  %value.src = getelementptr i8, ptr %value, i64 %vi
  %value.byte = load i8, ptr %value.src, align 1
  %value.base = add i64 %key.length, 1
  %value.offset = add i64 %value.base, %vi
  %value.dst = getelementptr i8, ptr %payload, i64 %value.offset
  store i8 %value.byte, ptr %value.dst, align 1
  %vi.next = add i64 %vi, 1
  br label %value.loop
terminate:
  %bytes = add i64 %sum, 1
  %end = getelementptr i8, ptr %payload, i64 %bytes
  store i8 0, ptr %end, align 1
  %written = call i64 @__ncc_fs_write(ptr %path, ptr %payload, i64 %bytes, i1 false)
  %synced = call i32 @__ncc_fs_sync(ptr %path)
  %write.ok = icmp eq i64 %written, %bytes
  %sync.ok = icmp ne i32 %synced, 0
  %ok = and i1 %write.ok, %sync.ok
  %released.payload = call i1 @__ncc_raw_aligned_free(ptr %payload)
  %released.path = call i1 @__ncc_raw_aligned_free(ptr %path)
  %result = zext i1 %ok to i32
  ret i32 %result
release.path.failure:
  %released.failed.path = call i1 @__ncc_raw_aligned_free(ptr %path)
  br label %failure
failure:
  ret i32 0
}

define ptr @ncc_rt_registry_read(ptr %key) {
entry:
  %path = call ptr @ncc.system.registry.path(ptr %key)
  %path.missing = icmp eq ptr %path, null
  br i1 %path.missing, label %empty, label %measure
measure:
  %length = call i64 @__ncc_fs_size(ptr %path)
  %valid = icmp sgt i64 %length, 0
  br i1 %valid, label %allocate, label %release.path.empty
allocate:
  %capacity = add i64 %length, 1
  %payload = call ptr @__ncc_raw_aligned_alloc(i64 %capacity, i64 16)
  %missing = icmp eq ptr %payload, null
  br i1 %missing, label %release.path.empty, label %read
read:
  %count = call i64 @__ncc_fs_read(ptr %path, ptr %payload, i64 %length)
  %released.path = call i1 @__ncc_raw_aligned_free(ptr %path)
  %read.ok = icmp eq i64 %count, %length
  br i1 %read.ok, label %terminate, label %release.payload.empty
terminate:
  %end = getelementptr i8, ptr %payload, i64 %count
  store i8 0, ptr %end, align 1
  %key.length = call i64 @ncc.system.strlen(ptr %key)
  br label %compare
compare:
  %index = phi i64 [ 0, %terminate ], [ %next, %compare.more ]
  %done = icmp eq i64 %index, %key.length
  br i1 %done, label %newline, label %compare.more
compare.more:
  %expected.ptr = getelementptr i8, ptr %key, i64 %index
  %actual.ptr = getelementptr i8, ptr %payload, i64 %index
  %expected = load i8, ptr %expected.ptr, align 1
  %actual = load i8, ptr %actual.ptr, align 1
  %same = icmp eq i8 %expected, %actual
  %next = add i64 %index, 1
  br i1 %same, label %compare, label %release.payload.empty
newline:
  %newline.ptr = getelementptr i8, ptr %payload, i64 %key.length
  %newline.byte = load i8, ptr %newline.ptr, align 1
  %newline.ok = icmp eq i8 %newline.byte, 10
  br i1 %newline.ok, label %allocate.value, label %release.payload.empty
allocate.value:
  %prefix = add i64 %key.length, 1
  %value.length = sub i64 %count, %prefix
  %value.capacity = add i64 %value.length, 1
  %value = call ptr @__ncc_raw_aligned_alloc(i64 %value.capacity, i64 16)
  %value.missing = icmp eq ptr %value, null
  br i1 %value.missing, label %release.payload.empty, label %copy.value
copy.value:
  br label %value.loop
value.loop:
  %vi = phi i64 [ 0, %copy.value ], [ %vi.next, %value.more ]
  %value.done = icmp eq i64 %vi, %value.length
  br i1 %value.done, label %value.terminate, label %value.more
value.more:
  %source.offset = add i64 %prefix, %vi
  %source = getelementptr i8, ptr %payload, i64 %source.offset
  %byte = load i8, ptr %source, align 1
  %destination = getelementptr i8, ptr %value, i64 %vi
  store i8 %byte, ptr %destination, align 1
  %vi.next = add i64 %vi, 1
  br label %value.loop
value.terminate:
  %value.end = getelementptr i8, ptr %value, i64 %value.length
  store i8 0, ptr %value.end, align 1
  %released.payload = call i1 @__ncc_raw_aligned_free(ptr %payload)
  ret ptr %value
release.path.empty:
  %released.empty.path = call i1 @__ncc_raw_aligned_free(ptr %path)
  br label %empty
release.payload.empty:
  %released.empty.payload = call i1 @__ncc_raw_aligned_free(ptr %payload)
  br label %empty
empty:
  %fallback = call ptr @ncc.system.empty()
  ret ptr %fallback
}
"#;

#[cfg(test)]
mod tests {
    use super::render_system_registry_runtime;

    #[test]
    fn system_registry_selects_material_os_abis_without_c_source() {
        let win64 = render_system_registry_runtime("x86_64-pc-windows-msvc", true, true)
            .expect("Win64 system runtime");
        assert!(win64.contains("@GetSystemTimes"));
        assert!(win64.contains("@RegCreateKeyExA"));
        assert!(win64.contains("define double @ncc_rt_system_cpu_load"));
        assert!(win64.contains("define ptr @ncc_rt_registry_read"));

        let win32 = render_system_registry_runtime("i686-pc-windows-msvc", true, true)
            .expect("Win32 system runtime");
        assert!(win32.contains("x86_stdcallcc"));
        assert!(win32.contains("\\01_RegQueryValueExA@24"));

        let linux = render_system_registry_runtime("x86_64-unknown-linux-gnu", true, true)
            .expect("Linux system runtime");
        assert!(linux.contains("/proc/stat"));
        assert!(linux.contains("@__ncc_fs_write"));
        assert!(linux.contains("@getenv"));
        assert!(!linux.contains("@__ncc_raw_aligned_alloc(i64 %capacity, i64 1)"));

        let darwin = render_system_registry_runtime("aarch64-apple-darwin", true, true)
            .expect("Darwin system runtime");
        assert!(darwin.contains("@host_statistics"));
        assert!(darwin.contains("@__ncc_fs_read"));
    }

    #[test]
    fn unsupported_freestanding_target_is_rejected_explicitly() {
        let error = render_system_registry_runtime("riscv64-unknown-none", true, true)
            .expect_err("freestanding system runtime must be rejected");
        assert!(error.contains("no direct operating-system ABI"));
    }
}
