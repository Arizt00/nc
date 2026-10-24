#[derive(Default, Debug, Clone)]
pub(crate) struct PrimitiveCliStandardOptions {
    pub(crate) json: bool,
    pub(crate) md: bool,
    pub(crate) no_color: bool,
    pub(crate) strict: bool,
    pub(crate) real: bool,
    pub(crate) hardware: bool,
    pub(crate) evidence: bool,
    pub(crate) evidence_path: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) environment: Option<String>,
    pub(crate) device: Option<String>,
}

pub(crate) fn primitive_help_text() -> &'static str {
    "Usage: ncc primitive <doctor|check|witness|map|lower|build|qemu|run|capture|verify|report|target|clean> [args]\n\
     Primitive terminal IDE for material hardware workflows.\n\
     Common options: --json --md --no-color --strict --real --hardware --evidence [path] --target <triple> --environment <name> --device <path>\n\
     Examples:\n\
       ncc primitive doctor --json\n\
       ncc primitive doctor --real --json\n\
       ncc primitive target x86_64-unknown-none --json\n\
       ncc primitive check examples/primitive/hardware_identity_uart.ncp --target x86_64-unknown-none --environment freestanding\n\
       ncc primitive witness examples/primitive/hardware_identity_uart.ncp --json\n\
       ncc primitive map examples/primitive/hardware_identity_uart.ncp --json\n\
       ncc primitive lower examples/primitive/hardware_identity_uart.ncp --json\n\
       ncc primitive lower examples/primitive/hardware_identity_uart.ncp --emit-bytes --emit-artifact --json\n\
       ncc primitive build examples/primitive/hardware_identity_uart.ncp --out target/nc/primitive/hardware_identity_uart.sov\n\
       ncc primitive qemu examples/primitive/hardware_identity_uart.ncp --expect \"nC primitive alive\"\n\
       ncc primitive run examples/primitive/real/uart_identity_real.ncp --target x86_64-linux-host-serial --device /dev/ttyS0 --json\n\
       ncc primitive capture examples/primitive/real/uart_identity_real.ncp --target x86_64-linux-host-serial --device /dev/ttyS0 --json\n\
       ncc primitive verify examples/primitive/real/uart_identity_real.ncp --evidence reports/primitive_real/run/uart_identity_real.json\n\
       ncc primitive report examples/primitive/hardware_identity_uart.ncp --md-out reports/primitive/hardware_identity_uart_report.md --json-out reports/primitive/hardware_identity_uart_report.json"
}

pub(crate) fn primitive_parse_standard_options(
    args: &[String],
) -> Result<(PrimitiveCliStandardOptions, Vec<String>), Box<dyn std::error::Error>> {
    let mut opts = PrimitiveCliStandardOptions::default();
    let mut rest = Vec::new();
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                opts.json = true;
                idx += 1;
            }
            "--md" => {
                opts.md = true;
                idx += 1;
            }
            "--no-color" => {
                opts.no_color = true;
                idx += 1;
            }
            "--strict" => {
                opts.strict = true;
                idx += 1;
            }
            "--real" => {
                opts.real = true;
                idx += 1;
            }
            "--hardware" => {
                opts.hardware = true;
                idx += 1;
            }
            "--evidence" => {
                opts.evidence = true;
                if let Some(value) = args.get(idx + 1) {
                    if !value.starts_with("--") {
                        opts.evidence_path = Some(value.clone());
                        idx += 2;
                    } else {
                        idx += 1;
                    }
                } else {
                    idx += 1;
                }
            }
            "--target" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("primitive syntax: --target requires a value")?;
                opts.target = Some(value.clone());
                idx += 2;
            }
            "--environment" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("primitive syntax: --environment requires a value")?;
                opts.environment = Some(value.clone());
                idx += 2;
            }
            "--device" => {
                let value = args
                    .get(idx + 1)
                    .ok_or("primitive syntax: --device requires a value")?;
                opts.device = Some(value.clone());
                idx += 2;
            }
            _ => {
                rest.push(args[idx].clone());
                idx += 1;
            }
        }
    }
    Ok((opts, rest))
}
