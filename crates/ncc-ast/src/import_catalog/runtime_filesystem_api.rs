use super::{RuntimeApiFamilySpec, RuntimeApiFunctionSpec};

fn function(name: &str, args: &[&str], return_ty: &str) -> RuntimeApiFunctionSpec {
    RuntimeApiFunctionSpec {
        name: name.to_string(),
        arg_types: args.iter().map(|arg| (*arg).to_string()).collect(),
        arity_min: args.len(),
        variadic: false,
        return_ty: return_ty.to_string(),
    }
}

pub(super) fn runtime_filesystem_api_family_specs() -> Vec<RuntimeApiFamilySpec> {
    vec![
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.fs".to_string(),
            module_path: "runtime.fs".to_string(),
            functions: vec![
                // Raw surface: storage, capacities and release remain caller-controlled.
                function("fsWriteFrom", &["str", "ptr<byte>", "u64", "bool"], "int"),
                function("fsReadInto", &["str", "ptr<byte>", "u64"], "int"),
                function("fsSize", &["str"], "int"),
                function("fsCopy", &["str", "str"], "int"),
                function("listDirInto", &["str", "ptr<byte>", "u64"], "int"),
                function("absolutePathInto", &["str", "ptr<byte>", "u64"], "int"),
                function("officialPathInto", &["str", "ptr<byte>", "u64"], "int"),
                function("fsMetadataInto", &["str", "ptr<byte>", "u64"], "int"),
                function("fsPermissionsInto", &["str", "ptr<byte>", "u64"], "int"),
                function("fsSetReadonly", &["str", "bool"], "bool"),
                function("fsLock", &["str", "bool", "bool"], "addr"),
                function("fsUnlock", &["addr"], "bool"),
                function("fsRelease", &["addr"], "bool"),
                function("fsTextRelease", &["Result<str, FsError>"], "bool"),
                function("fsBytesRelease", &["Result<array<byte>, FsError>"], "bool"),
                function("fsListRelease", &["Result<list<str>, FsError>"], "bool"),
                function("fsLinesRelease", &["ptr<byte>"], "bool"),
                // Source compatibility. Every memory-bearing return has an explicit releaser.
                function("readFile", &["str"], "Result<str, FsError>"),
                function("writeFile", &["str", "str"], "Result<bool, FsError>"),
                function("appendFile", &["str", "str"], "Result<bool, FsError>"),
                function("fsread", &["str"], "Result<str, FsError>"),
                function("fswrite", &["str", "str"], "Result<bool, FsError>"),
                function("fsappend", &["str", "str"], "Result<bool, FsError>"),
                function("fsync", &["str"], "Result<bool, FsError>"),
                function("exists", &["str"], "bool"),
                function("fexists", &["str"], "bool"),
                function("deleteFile", &["str"], "Result<bool, FsError>"),
                function("delete_file", &["str"], "bool"),
                function("renamePath", &["str", "str"], "Result<bool, FsError>"),
                function("movePath", &["str", "str"], "Result<bool, FsError>"),
                function("rename_file", &["str", "str"], "bool"),
                function("copyFile", &["str", "str"], "Result<int, FsError>"),
                function("createDir", &["str"], "Result<bool, FsError>"),
                function("removeDir", &["str"], "Result<bool, FsError>"),
                function("listDir", &["str"], "Result<list<str>, FsError>"),
                function("metadata", &["str"], "Result<str, FsError>"),
                function("permissions", &["str"], "Result<str, FsError>"),
                function("setReadonly", &["str", "bool"], "Result<bool, FsError>"),
                function("absolutePath", &["str"], "Result<str, FsError>"),
                function("officialPath", &["str"], "Result<str, FsError>"),
                function("lockFile", &["str"], "Result<FileLockHandle, FsError>"),
                function("unlockFile", &["FileLockHandle"], "Result<bool, FsError>"),
                function("readBytes", &["str"], "Result<array<byte>, FsError>"),
                function("fread_lines", &["str"], "array<str>"),
                function("fswriteResult", &["str", "str"], "Result<bool, FsError>"),
                function(
                    "readBytesInto",
                    &["str", "array<byte>"],
                    "Result<int, FsError>",
                ),
                function(
                    "writeBytes",
                    &["str", "array<byte>"],
                    "Result<int, FsError>",
                ),
            ],
        },
        RuntimeApiFamilySpec {
            import_path: "nC.runtime.dir".to_string(),
            module_path: "runtime.dir".to_string(),
            functions: vec![
                function("currentDirInto", &["ptr<byte>", "u64"], "int"),
                function("listDir", &["str"], "Result<list<str>, FsError>"),
                function("createDir", &["str"], "Result<bool, FsError>"),
                function("removeDir", &["str"], "Result<bool, FsError>"),
                function("currentDir", &[], "Result<str, FsError>"),
                function("changeDir", &["str"], "Result<bool, FsError>"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_surface_has_raw_buffers_and_explicit_compatibility_release() {
        let families = runtime_filesystem_api_family_specs();
        assert_eq!(families.len(), 2);
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsReadInto"
                && function.arg_types == ["str", "ptr<byte>", "u64"]
                && function.return_ty == "int"
        }));
        assert!(families[0]
            .functions
            .iter()
            .any(|function| function.name == "fsRelease"));
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsTextRelease"
                && function.arg_types == ["Result<str, FsError>"]
                && function.return_ty == "bool"
        }));
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsCopy"
                && function.arg_types == ["str", "str"]
                && function.return_ty == "int"
        }));
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsBytesRelease"
                && function.arg_types == ["Result<array<byte>, FsError>"]
                && function.return_ty == "bool"
        }));
        for name in ["absolutePathInto", "officialPathInto"] {
            assert!(families[0].functions.iter().any(|function| {
                function.name == name
                    && function.arg_types == ["str", "ptr<byte>", "u64"]
                    && function.return_ty == "int"
            }));
        }
        for name in ["fsMetadataInto", "fsPermissionsInto"] {
            assert!(families[0].functions.iter().any(|function| {
                function.name == name
                    && function.arg_types == ["str", "ptr<byte>", "u64"]
                    && function.return_ty == "int"
            }));
        }
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsSetReadonly"
                && function.arg_types == ["str", "bool"]
                && function.return_ty == "bool"
        }));
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsLock"
                && function.arg_types == ["str", "bool", "bool"]
                && function.return_ty == "addr"
        }));
        assert!(families[0].functions.iter().any(|function| {
            function.name == "fsUnlock"
                && function.arg_types == ["addr"]
                && function.return_ty == "bool"
        }));
        assert!(families[1].functions.iter().any(|function| {
            function.name == "currentDirInto"
                && function.arg_types == ["ptr<byte>", "u64"]
                && function.return_ty == "int"
        }));
    }
}
