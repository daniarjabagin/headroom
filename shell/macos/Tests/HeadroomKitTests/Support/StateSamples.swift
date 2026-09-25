enum StateSamples {
    static let legacyPeriod = #"{"cost_usd_micros":0,"total_tokens":0,"partial":false,"by_provider":[]}"#

    static let legacy = #"""
        {"version":1,"app_version":"0.5.1","generated_at":"2026-09-23T10:00:00Z","next_refresh_at":null,
         "last_success_at":null,"offline":false,
         "display":{"theme":"system","language":"en","value_mode":"used","reset_format":"countdown",
           "panel_label":"percent","show_spend":true,"show_account_spend":true,"show_trend":true,
           "show_forecast":true,"translucent":false,"hidden_windows":{}},
         "headline":{"account_id":"codex:work","provider":"codex","provider_name":"Codex","account_label":"Work",
           "window":"session","window_label":"Session","used_percent":55.0,"remaining_percent":45.0,"tone":"warning"},
         "accounts":[{"id":"codex:work","provider":"codex","provider_name":"Codex","label":"Work","email":null,
           "plan":"Pro","hidden":false,"owner":"cli","status":"fresh","error":null,"updated_at":null,"source":null,
           "windows":[],"balances":[],"notices":[],"usage_home":"~/.codex"}],
         "combined":[{"provider":"codex","provider_name":"Codex","account_ids":[],"accounts":[],"windows":[]}],
         "usage":[],
         "spend":{"today":\#(legacyPeriod),"yesterday":\#(legacyPeriod),"last_30_days":\#(legacyPeriod)}}
        """#

    static let spendPeriod = #"""
        {"cost_usd_micros":12400000,"total_tokens":57100000,"partial":true,"cost_per_mtok_usd_micros":217163,
         "by_provider":[],
         "projects":[{"project":"~/code/headroom","cost_usd_micros":9100000,"total_tokens":48100000,"partial":false,
           "share_permille":733,"cost_per_mtok_usd_micros":189189,"by_provider":[
             {"provider":"claude","provider_name":"Claude","cost_usd_micros":8000000,"total_tokens":40000000},
             {"provider":"codex","provider_name":"Codex","cost_usd_micros":1100000,"total_tokens":8100000}]}],
         "projects_other":{"count":4,"cost_usd_micros":3300000,"total_tokens":9000000,"partial":true,
           "share_permille":266}}
        """#
}
