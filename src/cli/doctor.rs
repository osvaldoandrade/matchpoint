use crate::engine::DoctorReport;

pub(super) fn lines(report: &DoctorReport) -> Vec<String> {
    let mut output = vec![if report.ok() {
        "LOCAL DIAGNOSTICS: OK".into()
    } else {
        "LOCAL DIAGNOSTICS: needs attention".into()
    }];
    for (name, ok, detail) in &report.checks {
        let text = match name.as_str() {
            "filesystem" if *ok => "Disk: local, suitable for jobs.".into(),
            "filesystem" => "Disk: unsuitable. Choose a local directory.".into(),
            "free_space" if *ok => format!("Free space: {detail}; sufficient."),
            "free_space" => format!("Space: {detail}. Free at least 256 MiB."),
            "sqlite" if *ok => "Local writes: OK.".into(),
            "sqlite" => "Write failed. Check space and permissions.".into(),
            "auth_config" if detail.starts_with("client_id present") => {
                "API: client ID saved; secret not read.".into()
            }
            "auth_config" => "API: no client ID. Configure before sending.".into(),
            "contract" => {
                "API responses: pending contract.\n2xx remains UNKNOWN; customer not confirmed."
                    .into()
            }
            "api_access" if !*ok => "API profile: invalid. Fix URLs/client ID.".into(),
            "api_access" if detail.starts_with("remote enabled") => {
                "Remote: enabled for saved URLs.".into()
            }
            "api_access" => "Remote: disabled.\nEnable in Settings to send.".into(),
            "keychain_probe" => "Keychain: not tested; no secret read.".into(),
            _ => format!("{name}: {} {detail}", if *ok { "OK" } else { "FAILED" }),
        };
        output.push(text);
        if !ok {
            output.push(format!("Technical detail: {detail}"));
        }
    }
    output
}
