//! Deep-link install routes for harnesses whose config is GUI-managed.
//!
//! VS Code (Copilot) does not expose a stable per-OS user config file, but it does
//! handle a `vscode:mcp/install?<urlencoded-json>` URI that triggers its own
//! "add MCP server" UI (with a consent prompt). For headless installs where no
//! desktop session can open the URI, we also produce a clickable
//! `https://vscode.dev/redirect/mcp/install?...` fallback to print.

use std::process::Command;

/// Both forms of the VS Code MCP install link for a given endpoint.
pub struct VsCodeLinks {
    /// `vscode:mcp/install?<encoded>` — opens VS Code directly.
    pub uri: String,
    /// `https://vscode.dev/redirect/mcp/install?...` — clickable web fallback.
    pub redirect: String,
}

/// Both forms of the Cursor MCP install link for a given endpoint.
pub struct CursorLinks {
    /// `cursor://anysphere.cursor-deeplink/mcp/install?...` — opens Cursor directly.
    pub uri: String,
    /// Web fallback for headless environments.
    pub redirect: String,
}

/// Build the VS Code install links for the `engrammic` HTTP MCP server at `endpoint`.
///
/// The native URI encodes a single server object that includes its `name`. The web
/// redirect form passes `name` separately and url-encodes the rest as `config`.
pub fn vscode_links(endpoint: &str) -> VsCodeLinks {
    // Built by hand (not serde) so the key order is stable and predictable in the URL.
    let server_obj = format!(
        r#"{{"name":"engrammic","type":"http","url":"{}"}}"#,
        endpoint
    );
    let config_obj = format!(r#"{{"type":"http","url":"{}"}}"#, endpoint);

    VsCodeLinks {
        uri: format!("vscode:mcp/install?{}", percent_encode(&server_obj)),
        redirect: format!(
            "https://vscode.dev/redirect/mcp/install?name=engrammic&config={}",
            percent_encode(&config_obj)
        ),
    }
}

/// Build the Cursor install links for the `engrammic` HTTP MCP server at `endpoint`.
pub fn cursor_links(endpoint: &str) -> CursorLinks {
    let config_obj = format!(r#"{{"type":"http","url":"{}"}}"#, endpoint);

    CursorLinks {
        uri: format!(
            "cursor://anysphere.cursor-deeplink/mcp/install?name=engrammic&config={}",
            percent_encode(&config_obj)
        ),
        redirect: format!(
            "https://cursor.com/redirect/mcp/install?name=engrammic&config={}",
            percent_encode(&config_obj)
        ),
    }
}

/// Attempt to open a URI with the OS handler. Returns `true` if the opener launched
/// (not a guarantee the URI was handled). Never errors — callers print the link too.
pub fn try_open(uri: &str) -> bool {
    // On Linux, opening a GUI handler is pointless without a display server.
    // Treat an unset OR empty var as "no display" (empty $DISPLAY is a real headless case).
    #[cfg(target_os = "linux")]
    {
        let has_display = |k| std::env::var_os(k).is_some_and(|v| !v.is_empty());
        if !has_display("DISPLAY") && !has_display("WAYLAND_DISPLAY") {
            return false;
        }
    }

    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(uri).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", uri]).status()
    } else {
        Command::new("xdg-open").arg(uri).status()
    };

    matches!(result, Ok(status) if status.success())
}

/// Percent-encode a string per RFC 3986, escaping everything except the unreserved
/// set (`A-Z a-z 0-9 - _ . ~`). Safe to apply to an entire JSON blob.
fn percent_encode(input: &str) -> String {
    fn is_unreserved(b: u8) -> bool {
        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
    }
    let mut out = String::with_capacity(input.len() * 3);
    for &b in input.as_bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_escapes_json_punctuation() {
        assert_eq!(percent_encode("a-z_0.9~"), "a-z_0.9~");
        assert_eq!(percent_encode("{}"), "%7B%7D");
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode(":/\""), "%3A%2F%22");
    }

    #[test]
    fn vscode_uri_round_trips_to_expected_json() {
        let links = vscode_links("https://beta.engrammic.ai/mcp/");
        assert!(links.uri.starts_with("vscode:mcp/install?"));
        // The encoded payload must decode back to the exact server object.
        let encoded = links.uri.strip_prefix("vscode:mcp/install?").unwrap();
        assert_eq!(
            percent_decode(encoded),
            r#"{"name":"engrammic","type":"http","url":"https://beta.engrammic.ai/mcp/"}"#
        );
        assert!(links
            .redirect
            .starts_with("https://vscode.dev/redirect/mcp/install?name=engrammic&config="));
    }

    // Test-only decoder to verify the encoder is lossless.
    fn percent_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                out.push(u8::from_str_radix(hex, 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }
}
