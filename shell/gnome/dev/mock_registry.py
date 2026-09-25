def cli_login(program):
    return {"kind": "cli_login", "program": program}


def api_key(console_url, hint=None):
    return {"kind": "api_key", "label": "API key", "console_url": console_url, "hint": hint}


def auto_detect(reason):
    return {"kind": "auto_detect", "reason": reason}


def links(status=None, dashboard=None, usage=None):
    return {"status": status, "dashboard": dashboard, "usage": usage}


def registry_entry(provider_id, name, methods, provider_links, local_usage=False):
    return {"id": provider_id, "display_name": name, "add_account": methods, "multi_account": True,
            "local_usage": local_usage, "links": provider_links}


PROVIDERS = [
    registry_entry("codex", "Codex", [cli_login("codex")],
                   links("https://status.openai.com", "https://chatgpt.com/codex"), True),
    registry_entry("claude", "Claude", [cli_login("claude")],
                   links("https://status.claude.com", "https://claude.ai"), True),
    registry_entry("opencode", "OpenCode", [auto_detect("Headroom reads OpenCode's local logs, nothing to add.")],
                   links(dashboard="https://opencode.ai/zen"), True),
    registry_entry("openrouter", "OpenRouter", [api_key("https://openrouter.ai/settings/keys", "Starts with sk-or-")],
                   links("https://status.openrouter.ai", "https://openrouter.ai/settings/credits",
                         "https://openrouter.ai/activity")),
    registry_entry("zai", "Z.ai", [api_key("https://z.ai/manage-apikey/apikey-list")],
                   links(None, "https://z.ai/manage-apikey/coding-plan/personal/my-plan",
                         "https://z.ai/manage-apikey/billing")),
    registry_entry("kimi", "Kimi", [cli_login("kimi"), api_key("https://platform.moonshot.ai/console/api-keys")],
                   links("https://status.moonshot.cn", "https://www.kimi.com/code/console",
                         "https://www.kimi.com/code/console")),
    registry_entry("minimax", "MiniMax", [api_key("https://platform.minimax.io/user-center/basic-information")],
                   links("https://status.minimax.io", "https://platform.minimax.io/console/plan",
                         "https://platform.minimax.io/console/plan")),
    registry_entry("grok", "Grok", [api_key("https://console.x.ai", "Starts with xai-")],
                   links("https://status.x.ai", "https://console.x.ai")),
    registry_entry("cline", "Cline", [auto_detect("Headroom finds the Cline sign-in in VS Code's storage.")],
                   links("https://status.cline.bot", "https://app.cline.bot/dashboard",
                         "https://app.cline.bot/dashboard")),
    registry_entry("devin", "Devin", [api_key("https://app.devin.ai/settings/api-keys")],
                   links("https://www.devinstatus.com", "https://app.devin.ai")),
    registry_entry("copilot", "GitHub Copilot", [cli_login("gh")],
                   links("https://www.githubstatus.com", "https://github.com/settings/copilot",
                         "https://github.com/settings/billing/summary")),
    registry_entry("cursor", "Cursor",
                   [auto_detect("Headroom reads the sign-in of the Cursor app on this computer.")],
                   links("https://status.cursor.com", "https://cursor.com/dashboard",
                         "https://cursor.com/dashboard/usage")),
    registry_entry("antigravity", "Antigravity", [auto_detect("Headroom reads the sign-in of the Antigravity app.")],
                   links(dashboard="https://antigravity.google")),
    registry_entry("ollama", "Ollama", [api_key("https://ollama.com/settings/keys")],
                   links(None, "https://ollama.com/settings", "https://ollama.com/settings")),
    registry_entry("kilo", "Kilo Code", [cli_login("kilo"), api_key("https://app.kilo.ai/profile"),
                                         auto_detect("Found when you sign in with `kilo auth login`")],
                   links("https://status.kilo.ai", "https://app.kilo.ai", "https://app.kilo.ai/usage")),
    registry_entry("warp", "Warp", [api_key("https://docs.warp.dev/reference/cli/api-keys", "Starts with wk-")],
                   links("https://status.warp.dev", "https://app.warp.dev")),
    registry_entry("poe", "Poe", [api_key("https://poe.com/api/keys")],
                   links("https://status.poe.com", "https://poe.com/settings", "https://poe.com/points_history")),
    registry_entry("deepseek", "DeepSeek", [api_key("https://platform.deepseek.com/api_keys")],
                   links("https://status.deepseek.com", "https://platform.deepseek.com",
                         "https://platform.deepseek.com/usage")),
    registry_entry("moonshot", "Moonshot API", [api_key("https://platform.kimi.ai/console/api-keys")],
                   links("https://status.moonshot.cn", "https://platform.kimi.ai/console",
                         "https://platform.kimi.ai/console/account")),
]
PROVIDER_NAMES = {entry["id"]: entry["display_name"] for entry in PROVIDERS}
PROVIDER_ORDER = [entry["id"] for entry in PROVIDERS]
