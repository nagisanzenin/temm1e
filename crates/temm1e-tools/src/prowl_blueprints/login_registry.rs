//! Login URL registry for well-known services.
//! Maps short service names to their login page URLs.
//!
//! Usage:
//! - `/login facebook` → looks up "facebook" → https://www.facebook.com/login
//! - `/login https://custom-app.com/signin` → uses URL directly
//! - `/login myapp https://myapp.com/auth` → custom name + URL

use std::collections::HashMap;

/// Get the login URL for a known service, or None if not in registry.
pub fn lookup_login_url(service: &str) -> Option<&'static str> {
    KNOWN_SERVICES.get(service.to_lowercase().as_str()).copied()
}

/// Try to resolve a /login argument into (service_name, login_url).
///
/// Supports:
/// - `/login facebook` → ("facebook", "https://www.facebook.com/login")
/// - `/login https://example.com/login` → ("example.com", "https://example.com/login")
/// - `/login myapp https://myapp.com/auth` → ("myapp", "https://myapp.com/auth")
pub fn resolve_login_args(args: &str) -> Option<(String, String)> {
    let args = args.trim();
    if args.is_empty() {
        return None;
    }

    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    match parts.len() {
        1 => {
            let arg = parts[0];
            // Check if it's a URL
            if arg.starts_with("http://") || arg.starts_with("https://") {
                // Extract domain as service name
                let name = extract_domain(arg).unwrap_or_else(|| arg.to_string());
                Some((name, arg.to_string()))
            } else {
                // Look up as service name
                if let Some(url) = lookup_login_url(arg) {
                    Some((arg.to_lowercase(), url.to_string()))
                } else {
                    // Fallback: try https://{service}.com/login
                    let url = format!("https://www.{}.com/login", arg.to_lowercase());
                    Some((arg.to_lowercase(), url))
                }
            }
        }
        2 => {
            let name = parts[0];
            let url = parts[1];
            if url.starts_with("http://") || url.starts_with("https://") {
                Some((name.to_lowercase(), url.to_string()))
            } else {
                // Second arg is not a URL, treat whole thing as service name
                if let Some(url) = lookup_login_url(args) {
                    Some((args.to_lowercase(), url.to_string()))
                } else {
                    let url = format!("https://www.{}.com/login", name.to_lowercase());
                    Some((name.to_lowercase(), url))
                }
            }
        }
        _ => None,
    }
}

fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let domain = without_scheme.split('/').next()?;
    let clean = domain
        .strip_prefix("www.")
        .unwrap_or(domain)
        .split('.')
        .next()?;
    Some(clean.to_string())
}

// 100 most well-known services with their login URLs
static KNOWN_SERVICES: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();

        // ── Social Media ──────────────────────────────────────
        m.insert("facebook", "https://www.facebook.com/login");
        m.insert("fb", "https://www.facebook.com/login");
        m.insert("instagram", "https://www.instagram.com/accounts/login/");
        m.insert("ig", "https://www.instagram.com/accounts/login/");
        m.insert("twitter", "https://x.com/i/flow/login");
        m.insert("x", "https://x.com/i/flow/login");
        m.insert("tiktok", "https://www.tiktok.com/login");
        m.insert("linkedin", "https://www.linkedin.com/login");
        m.insert("reddit", "https://www.reddit.com/login/");
        m.insert("pinterest", "https://www.pinterest.com/login/");
        m.insert(
            "snapchat",
            "https://accounts.snapchat.com/accounts/v2/login",
        );
        m.insert("threads", "https://www.threads.net/login");
        m.insert("tumblr", "https://www.tumblr.com/login");
        m.insert("mastodon", "https://mastodon.social/auth/sign_in");

        // ── Messaging ─────────────────────────────────────────
        m.insert("telegram", "https://web.telegram.org/");
        m.insert("whatsapp", "https://web.whatsapp.com/");
        m.insert("discord", "https://discord.com/login");
        m.insert("slack", "https://slack.com/signin");
        m.insert("teams", "https://teams.microsoft.com/");
        m.insert("messenger", "https://www.messenger.com/login/");
        m.insert("signal", "https://signal.org/download/");
        m.insert("zalo", "https://chat.zalo.me/");
        m.insert("wechat", "https://web.wechat.com/");
        m.insert("viber", "https://account.viber.com/login");
        m.insert("skype", "https://login.skype.com/login");

        // ── Google ────────────────────────────────────────────
        m.insert("google", "https://accounts.google.com/signin");
        m.insert("gmail", "https://accounts.google.com/signin");
        m.insert("youtube", "https://accounts.google.com/signin");
        m.insert("gdrive", "https://accounts.google.com/signin");
        m.insert("gcloud", "https://console.cloud.google.com/");

        // ── Microsoft ─────────────────────────────────────────
        m.insert("microsoft", "https://login.microsoftonline.com/");
        m.insert("outlook", "https://outlook.live.com/owa/");
        m.insert("hotmail", "https://outlook.live.com/owa/");
        m.insert("office365", "https://login.microsoftonline.com/");
        m.insert("azure", "https://portal.azure.com/");
        m.insert("onedrive", "https://onedrive.live.com/login");

        // ── Apple ─────────────────────────────────────────────
        m.insert("apple", "https://appleid.apple.com/sign-in");
        m.insert("icloud", "https://www.icloud.com/");

        // ── Developer / Code ──────────────────────────────────
        m.insert("github", "https://github.com/login");
        m.insert("gh", "https://github.com/login");
        m.insert("gitlab", "https://gitlab.com/users/sign_in");
        m.insert("bitbucket", "https://bitbucket.org/account/signin/");
        m.insert("stackoverflow", "https://stackoverflow.com/users/login");
        m.insert("npm", "https://www.npmjs.com/login");
        m.insert("docker", "https://hub.docker.com/sso/start");
        m.insert("vercel", "https://vercel.com/login");
        m.insert("netlify", "https://app.netlify.com/login");
        m.insert("heroku", "https://id.heroku.com/login");
        m.insert("aws", "https://signin.aws.amazon.com/signin");
        m.insert("digitalocean", "https://cloud.digitalocean.com/login");
        m.insert("cloudflare", "https://dash.cloudflare.com/login");
        m.insert("render", "https://dashboard.render.com/login");
        m.insert("railway", "https://railway.app/login");
        m.insert("supabase", "https://supabase.com/dashboard/sign-in");
        m.insert("firebase", "https://console.firebase.google.com/");
        m.insert("replit", "https://replit.com/login");
        m.insert("codepen", "https://codepen.io/login");
        m.insert("figma", "https://www.figma.com/login");
        m.insert("notion", "https://www.notion.so/login");

        // ── Shopping / E-commerce ─────────────────────────────
        m.insert("amazon", "https://www.amazon.com/ap/signin");
        m.insert("ebay", "https://signin.ebay.com/ws/eBayISAPI.dll?SignIn");
        m.insert("shopify", "https://accounts.shopify.com/store-login");
        m.insert("etsy", "https://www.etsy.com/signin");
        m.insert("walmart", "https://www.walmart.com/account/login");
        m.insert("aliexpress", "https://login.aliexpress.com/");
        m.insert("shopee", "https://shopee.com/buyer/login");
        m.insert("lazada", "https://member.lazada.com/user/login");
        m.insert("tiki", "https://tiki.vn/login");

        // ── Entertainment / Streaming ─────────────────────────
        m.insert("netflix", "https://www.netflix.com/login");
        m.insert("spotify", "https://accounts.spotify.com/login");
        m.insert("twitch", "https://www.twitch.tv/login");
        m.insert("hulu", "https://auth.hulu.com/web/login");
        m.insert("disneyplus", "https://www.disneyplus.com/login");
        m.insert("disney", "https://www.disneyplus.com/login");
        m.insert("hbo", "https://play.hbomax.com/signIn");
        m.insert("primevideo", "https://www.primevideo.com/auth/signin");
        m.insert("applemusic", "https://music.apple.com/login");
        m.insert("soundcloud", "https://soundcloud.com/signin");
        m.insert("crunchyroll", "https://www.crunchyroll.com/login");

        // ── Productivity ──────────────────────────────────────
        m.insert("trello", "https://trello.com/login");
        m.insert("asana", "https://app.asana.com/-/login");
        m.insert("jira", "https://id.atlassian.com/login");
        m.insert("atlassian", "https://id.atlassian.com/login");
        m.insert("confluence", "https://id.atlassian.com/login");
        m.insert("monday", "https://auth.monday.com/");
        m.insert("clickup", "https://app.clickup.com/login");
        m.insert("todoist", "https://todoist.com/auth/login");
        m.insert("evernote", "https://www.evernote.com/Login.action");
        m.insert("airtable", "https://airtable.com/login");
        m.insert("miro", "https://miro.com/login/");
        m.insert("canva", "https://www.canva.com/login/");
        m.insert("dropbox", "https://www.dropbox.com/login");
        m.insert("box", "https://account.box.com/login");
        m.insert("zoom", "https://zoom.us/signin");

        // ── Finance / Banking ─────────────────────────────────
        m.insert("paypal", "https://www.paypal.com/signin");
        m.insert("stripe", "https://dashboard.stripe.com/login");
        m.insert("wise", "https://wise.com/login");
        m.insert("revolut", "https://app.revolut.com/start");
        m.insert("coinbase", "https://www.coinbase.com/signin");
        m.insert("binance", "https://accounts.binance.com/en/login");
        m.insert("robinhood", "https://robinhood.com/login");

        // ── Education ─────────────────────────────────────────
        m.insert("coursera", "https://www.coursera.org/login");
        m.insert("udemy", "https://www.udemy.com/join/login-popup/");
        m.insert("duolingo", "https://www.duolingo.com/log-in");
        m.insert("khan", "https://www.khanacademy.org/login");

        // ── AI / Tools ────────────────────────────────────────
        m.insert("chatgpt", "https://chat.openai.com/auth/login");
        m.insert("openai", "https://chat.openai.com/auth/login");
        m.insert("claude", "https://claude.ai/login");
        m.insert("anthropic", "https://console.anthropic.com/login");
        m.insert("midjourney", "https://www.midjourney.com/signin");
        m.insert("huggingface", "https://huggingface.co/login");

        m
    });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_service() {
        assert_eq!(
            lookup_login_url("facebook"),
            Some("https://www.facebook.com/login")
        );
        assert_eq!(lookup_login_url("github"), Some("https://github.com/login"));
        assert_eq!(
            lookup_login_url("FACEBOOK"),
            Some("https://www.facebook.com/login")
        ); // lookup is case-insensitive (to_lowercase before map lookup)
    }

    #[test]
    fn resolve_service_name() {
        let (name, url) = resolve_login_args("facebook").unwrap();
        assert_eq!(name, "facebook");
        assert_eq!(url, "https://www.facebook.com/login");
    }

    #[test]
    fn resolve_alias() {
        let (name, url) = resolve_login_args("fb").unwrap();
        assert_eq!(name, "fb");
        assert_eq!(url, "https://www.facebook.com/login");
    }

    #[test]
    fn resolve_direct_url() {
        let (name, url) = resolve_login_args("https://myapp.com/signin").unwrap();
        assert_eq!(name, "myapp");
        assert_eq!(url, "https://myapp.com/signin");
    }

    #[test]
    fn resolve_name_and_url() {
        let (name, url) = resolve_login_args("myapp https://myapp.com/auth").unwrap();
        assert_eq!(name, "myapp");
        assert_eq!(url, "https://myapp.com/auth");
    }

    #[test]
    fn resolve_unknown_falls_back() {
        let (name, url) = resolve_login_args("unknownservice").unwrap();
        assert_eq!(name, "unknownservice");
        assert_eq!(url, "https://www.unknownservice.com/login");
    }

    #[test]
    fn resolve_empty_returns_none() {
        assert!(resolve_login_args("").is_none());
        assert!(resolve_login_args("   ").is_none());
    }

    #[test]
    fn registry_has_100_services() {
        // We should have at least 100 entries (including aliases)
        assert!(
            KNOWN_SERVICES.len() >= 100,
            "Registry has {} entries",
            KNOWN_SERVICES.len()
        );
    }
}
