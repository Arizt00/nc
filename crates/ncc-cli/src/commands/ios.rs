use super::super::*;
use serde_json::json;
use std::path::{Path, PathBuf};

const DEFAULT_APP_BUNDLE_ID: &str = "com.aristote.uncall";
const DEFAULT_EXTENSION_BUNDLE_ID: &str = "com.aristote.uncall.CallDirectory";
const DEFAULT_APP_GROUP: &str = "group.com.aristote.uncall";

pub(crate) fn run_ios_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty()
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        println!("{}", ios_help());
        return Ok(());
    }
    match args[0].as_str() {
        "identity" | "handoff" => ios_identity(&args[1..]),
        other => Err(format!("unknown ios command '{other}'\n{}", ios_help()).into()),
    }
}

fn ios_help() -> &'static str {
    "Usage: ncc ios identity <file.n> [--out-dir <dir>] [--bundle-id <id>] [--extension-bundle-id <id>] [--app-group <group.id>] [--team-id <10-character-id>] [--json]\n\
     Generates the validated Xcode identity handoff, Info.plist files and App Group entitlements. The Team ID can also arrive through NCC_APPLE_TEAM_ID. It never claims to create a signed IPA; archiving still requires macOS, Xcode, registered identifiers, certificates and provisioning profiles."
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn positional(args: &[String]) -> Option<String> {
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--out-dir" | "--bundle-id" | "--extension-bundle-id" | "--app-group" | "--team-id"
        ) {
            skip = true;
        } else if !arg.starts_with('-') {
            return Some(arg.clone());
        }
    }
    None
}

fn validate_apple_identifier(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let segments = value.split('.').collect::<Vec<_>>();
    if segments.len() < 3 {
        return Err(format!("Apple identifier '{value}' needs at least three segments").into());
    }
    for segment in segments {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(format!(
                "invalid Apple identifier '{value}'; use letters, digits, hyphens and periods"
            )
            .into());
        }
    }
    Ok(())
}

fn validate_identity(
    bundle_id: &str,
    extension_bundle_id: &str,
    app_group: &str,
    team_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_apple_identifier(bundle_id)?;
    validate_apple_identifier(extension_bundle_id)?;
    if extension_bundle_id == bundle_id
        || !extension_bundle_id.starts_with(&format!("{bundle_id}."))
    {
        return Err(
            "Call Directory bundle ID must be a child of the containing app bundle ID".into(),
        );
    }
    let group_suffix = app_group
        .strip_prefix("group.")
        .ok_or("App Group identifier must begin with 'group.'")?;
    validate_apple_identifier(group_suffix)?;
    if group_suffix != bundle_id {
        return Err("App Group suffix must match the containing app bundle ID".into());
    }
    if let Some(team) = team_id {
        if team.len() != 10
            || !team
                .chars()
                .all(|value| value.is_ascii_uppercase() || value.is_ascii_digit())
        {
            return Err(
                "Apple Team ID must contain exactly 10 uppercase ASCII letters or digits".into(),
            );
        }
    }
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn plist_header() -> &'static str {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"https://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n"
}

fn entitlements(app_group: &str) -> String {
    format!(
        "{}<dict>\n    <key>com.apple.security.application-groups</key>\n    <array>\n        <string>{}</string>\n    </array>\n</dict>\n</plist>\n",
        plist_header(),
        xml_escape(app_group)
    )
}

fn app_info_plist() -> String {
    format!(
        "{}<dict>\n    <key>CFBundleDisplayName</key><string>UnCall</string>\n    <key>CFBundleExecutable</key><string>$(EXECUTABLE_NAME)</string>\n    <key>CFBundleIdentifier</key><string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>\n    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\n    <key>CFBundleName</key><string>$(PRODUCT_NAME)</string>\n    <key>CFBundlePackageType</key><string>APPL</string>\n    <key>CFBundleShortVersionString</key><string>1.6.0</string>\n    <key>CFBundleVersion</key><string>2608.9.1</string>\n    <key>LSRequiresIPhoneOS</key><true/>\n    <key>MinimumOSVersion</key><string>15.0</string>\n    <key>UIApplicationSupportsIndirectInputEvents</key><true/>\n    <key>UILaunchScreen</key><dict/>\n</dict>\n</plist>\n",
        plist_header()
    )
}

fn extension_info_plist() -> String {
    format!(
        "{}<dict>\n    <key>CFBundleDisplayName</key><string>UnCall Call Directory</string>\n    <key>CFBundleExecutable</key><string>$(EXECUTABLE_NAME)</string>\n    <key>CFBundleIdentifier</key><string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>\n    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>\n    <key>CFBundleName</key><string>$(PRODUCT_NAME)</string>\n    <key>CFBundlePackageType</key><string>XPC!</string>\n    <key>CFBundleShortVersionString</key><string>1.6.0</string>\n    <key>CFBundleVersion</key><string>2608.9.1</string>\n    <key>NSExtension</key>\n    <dict>\n        <key>NSExtensionPointIdentifier</key><string>com.apple.callkit.call-directory</string>\n        <key>NSExtensionPrincipalClass</key><string>$(PRODUCT_MODULE_NAME).CallDirectoryProvider</string>\n    </dict>\n</dict>\n</plist>\n",
        plist_header()
    )
}

fn identity_xcconfig(
    bundle_id: &str,
    extension_bundle_id: &str,
    app_group: &str,
    team_id: Option<&str>,
) -> String {
    format!(
        "UNCALL_APP_BUNDLE_ID = {bundle_id}\nUNCALL_EXTENSION_BUNDLE_ID = {extension_bundle_id}\nUNCALL_APP_GROUP = {app_group}\nUNCALL_DEVELOPMENT_TEAM = {}\nCODE_SIGN_STYLE = Automatic\nIPHONEOS_DEPLOYMENT_TARGET = 15.0\nSUPPORTED_PLATFORMS = iphoneos iphonesimulator\n",
        team_id.unwrap_or("")
    )
}

fn app_target_xcconfig() -> &'static str {
    "#include \"UnCall.Identity.xcconfig\"\n\
PRODUCT_BUNDLE_IDENTIFIER = $(UNCALL_APP_BUNDLE_ID)\n\
DEVELOPMENT_TEAM = $(UNCALL_DEVELOPMENT_TEAM)\n\
CODE_SIGN_ENTITLEMENTS = UnCall.entitlements\n\
INFOPLIST_FILE = UnCall.Info.plist\n"
}

fn extension_target_xcconfig() -> &'static str {
    "#include \"UnCall.Identity.xcconfig\"\n\
PRODUCT_BUNDLE_IDENTIFIER = $(UNCALL_EXTENSION_BUNDLE_ID)\n\
DEVELOPMENT_TEAM = $(UNCALL_DEVELOPMENT_TEAM)\n\
CODE_SIGN_ENTITLEMENTS = UnCallCallDirectory.entitlements\n\
INFOPLIST_FILE = UnCallCallDirectory.Info.plist\n\
APPLICATION_EXTENSION_API_ONLY = YES\n"
}

fn ios_identity(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let source = positional(args).ok_or("ios identity requires the nC Call Directory source")?;
    let source = fs::canonicalize(PathBuf::from(source))?;
    if source.extension().and_then(|value| value.to_str()) != Some("n") {
        return Err("ios identity accepts a normal high-level .n source".into());
    }
    let ast = load_ast_from_path(&source.to_string_lossy())?;
    sema_check_with_warnings(&ast)?;
    let source_text = fs::read_to_string(&source)?;
    if !source_text.contains("CallKit.CXCallDirectoryProvider") {
        return Err(
            "nC source does not declare the official Call Directory platform contract".into(),
        );
    }

    let bundle_id =
        option_value(args, "--bundle-id").unwrap_or_else(|| DEFAULT_APP_BUNDLE_ID.to_string());
    let extension_bundle_id = option_value(args, "--extension-bundle-id")
        .unwrap_or_else(|| DEFAULT_EXTENSION_BUNDLE_ID.to_string());
    let app_group =
        option_value(args, "--app-group").unwrap_or_else(|| DEFAULT_APP_GROUP.to_string());
    let team_id = option_value(args, "--team-id").or_else(|| {
        env::var("NCC_APPLE_TEAM_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
    });
    validate_identity(
        &bundle_id,
        &extension_bundle_id,
        &app_group,
        team_id.as_deref(),
    )?;

    let out_dir = option_value(args, "--out-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            source
                .parent()
                .unwrap_or(Path::new("."))
                .join(".ncc")
                .join("ios")
                .join("identity")
        });
    let out_dir = absolute_path(out_dir)?;
    fs::create_dir_all(&out_dir)?;
    fs::write(
        out_dir.join("UnCall.Identity.xcconfig"),
        identity_xcconfig(
            &bundle_id,
            &extension_bundle_id,
            &app_group,
            team_id.as_deref(),
        ),
    )?;
    fs::write(out_dir.join("UnCall.App.xcconfig"), app_target_xcconfig())?;
    fs::write(
        out_dir.join("UnCall.CallDirectory.xcconfig"),
        extension_target_xcconfig(),
    )?;
    fs::write(
        out_dir.join("UnCall.entitlements"),
        entitlements(&app_group),
    )?;
    fs::write(
        out_dir.join("UnCallCallDirectory.entitlements"),
        entitlements(&app_group),
    )?;
    fs::write(out_dir.join("UnCall.Info.plist"), app_info_plist())?;
    fs::write(
        out_dir.join("UnCallCallDirectory.Info.plist"),
        extension_info_plist(),
    )?;

    let status = if team_id.is_some() {
        "identity_handoff_ready"
    } else {
        "awaiting_apple_team_id"
    };
    let manifest = json!({
        "schema": "nc.ios.identity.v1",
        "status": status,
        "source": source,
        "source_sha256": crate::content_cache::sha256_bytes(source_text.as_bytes()),
        "containing_app_bundle_id": bundle_id,
        "call_directory_extension_bundle_id": extension_bundle_id,
        "app_group": app_group,
        "development_team": team_id,
        "minimum_ios": "15.0",
        "official_extension_point": "com.apple.callkit.call-directory",
        "signed_ipa": false,
        "ipa_blockers": if status == "awaiting_apple_team_id" {
            vec!["macOS with Xcode", "Apple Team ID", "registered App IDs and App Group", "distribution certificate and provisioning profiles"]
        } else {
            vec!["macOS with Xcode", "registered App IDs and App Group", "distribution certificate and provisioning profiles"]
        },
        "generated_files": [
            "UnCall.Identity.xcconfig",
            "UnCall.App.xcconfig",
            "UnCall.CallDirectory.xcconfig",
            "UnCall.entitlements",
            "UnCallCallDirectory.entitlements",
            "UnCall.Info.plist",
            "UnCallCallDirectory.Info.plist"
        ]
    });
    let manifest_path = out_dir.join("UnCall.Identity.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    if has_flag(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!(
            "OK: iOS identity={} app={} extension={} app-group={} team={} ipa=not-generated",
            manifest_path.display(),
            bundle_id,
            extension_bundle_id,
            app_group,
            team_id.as_deref().unwrap_or("required-on-macos")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitive_uncall_identity_is_consistent() {
        validate_identity(
            DEFAULT_APP_BUNDLE_ID,
            DEFAULT_EXTENSION_BUNDLE_ID,
            DEFAULT_APP_GROUP,
            None,
        )
        .unwrap();
        assert!(validate_identity(
            DEFAULT_APP_BUNDLE_ID,
            DEFAULT_APP_BUNDLE_ID,
            DEFAULT_APP_GROUP,
            None
        )
        .is_err());
        assert!(validate_identity(
            DEFAULT_APP_BUNDLE_ID,
            DEFAULT_EXTENSION_BUNDLE_ID,
            "group.com.other.app",
            None
        )
        .is_err());
        assert!(validate_identity(
            DEFAULT_APP_BUNDLE_ID,
            DEFAULT_EXTENSION_BUNDLE_ID,
            DEFAULT_APP_GROUP,
            Some("ABC1234XYZ")
        )
        .is_ok());
    }

    #[test]
    fn generated_handoff_uses_call_directory_and_app_group() {
        let app = entitlements(DEFAULT_APP_GROUP);
        let extension = extension_info_plist();
        let app_config = app_target_xcconfig();
        let extension_config = extension_target_xcconfig();
        assert!(app.contains("com.apple.security.application-groups"));
        assert!(app.contains(DEFAULT_APP_GROUP));
        assert!(extension.contains("com.apple.callkit.call-directory"));
        assert!(extension.contains("CallDirectoryProvider"));
        assert!(app_config.contains("$(UNCALL_APP_BUNDLE_ID)"));
        assert!(extension_config.contains("$(UNCALL_EXTENSION_BUNDLE_ID)"));
        assert!(extension_config.contains("APPLICATION_EXTENSION_API_ONLY = YES"));
    }
}
