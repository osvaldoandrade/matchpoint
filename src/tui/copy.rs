//! English presentation catalog; file names and historic event data remain user data.
pub const HOME_TITLE: &str = " Parter Get Parter ";
pub const HOME_HEADLINE: &str = "Parter Get Parter";
pub const HOME_SUBHEADLINE: &str = "Local dataset matching";
pub const HOME_LINKS: [&str; 3] = ["NEW JOB", "JOBS", "SETTINGS"];
pub const HOME_KEYS: [&str; 3] = ["N", "J", "S"];
pub const HOME_COMMANDS: [&str; 3] = ["import-form", "jobs-view", "settings-view"];
pub const HOME_DESCRIPTIONS: [&str; 3] = [
    "Import CSV and prepare a new job.",
    "Review jobs, batches and results.",
    "Configure APIs, secrets and UI.",
];
pub const HOME_FOOTER: &str = " ←→ select · Enter open · Q quit\t/ commands · ? help";
pub const HOME_FOOTER_COMPACT: &str = " ←→ · Enter · Q quit\t/ · ? help";
pub const SETTINGS_TABS: [&str; 4] = ["API", "Secrets", "Processing", "Appearance"];
pub const JOB_TABS: [&str; 4] = ["Overview", "Batches", "Events", "Integrity"];
pub const API_FIELDS: [&str; 5] = [
    "Client ID",
    "Account-opening URL",
    "OAuth token URL",
    "Client secret (hidden)",
    "Allow remote requests",
];
pub const VALIDATION: [&str; 3] = ["11 digits", "Check digits", "None"];
pub const SPEED: [&str; 4] = ["Off (reduced motion)", "Slow", "Normal", "Fast"];

pub const HELP_PAGES: [&str;4] = [
"HOME\nN New Job · J Jobs · S Settings\nSplash: Enter opens the workspace.\n\nJOBS\nArrows select · Enter opens details\nTab switches detail tabs.\nR run/resume · P pause active job\nT threads · K checksum · E export\nH home · S Settings · Q quit",
"PROCESSING\nThreads: 1–256 concurrent calls.\nReserved includes the worker queue.\nRPS separately limits sends/second.\nP pauses the active job. Wait for Paused before T changes threads.\n\nProcessed = succeeded + final failure + UNKNOWN.\nUNKNOWN is uncertain, never a confirmed customer.\nQ/Ctrl+C waits for requests and writes.",
"INTEGRITY\nK checks stored input.csv and exports.\nCompare expected/actual SHA-256.\nMismatch: restore or create a new job.\nE chooses CSV/Parquet and partial export.\n\nRECOVERY\nEvents contains complete diagnostics.\n/retry-failed retries final failures.\njob requeue-unknown <id> may duplicate POST.\nPending contract: 2xx remains UNKNOWN.",
"SETTINGS\nAPI · Secrets · Processing · Appearance\nLeft/Right on tabs · Tab edits fields\nF2 changes tab · Ctrl+S saves group\nAPI + Secrets save together.\n* marks unsaved edits.\nEsc returns and discards unsaved edits.\nFonts apply to a new Ghostty window.\nCmd +/− resizes · Cmd 0 resets.\n\nADVANCED COMMANDS\n/ opens · Tab completes · Enter submits\nEsc clears · Ctrl+U clears field\nLeft/Right/Home/End/Delete edit input"
];
