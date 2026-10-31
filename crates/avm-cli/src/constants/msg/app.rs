//! App (shell-init, new plugin template) messages.

pub const PLUGIN_TEMPLATE_INTRO: &str = "Start a new plugin from the template repo:";
pub const PLUGIN_TEMPLATE_CMD: &str =
    "  gh repo create avm-plugin-{name} --template PrajaNova/avm-plugin-template --public --clone";
pub const APP_CMD_NO_INIT_HOOK: &str =
    "cmd has no init hook; run `avm-bin env --shell cmd` and use `avm-bin` directly";
