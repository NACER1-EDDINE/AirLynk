//! QR code for the session URL (FR-2, FR-3).

use qrcode::render::svg;
use qrcode::QrCode;

/// Render the session URL as an SVG QR code — the courier label face
/// (DESIGN.md). The URL carries the token in the path and the AES key in the
/// fragment; the QR only ever contains the full URL (SEC-13).
pub fn session_url_svg(url: &str) -> Result<String, qrcode::types::QrError> {
    let code = QrCode::new(url.as_bytes())?;
    Ok(code
        .render::<svg::Color>()
        .min_dimensions(2, 2)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_valid_svg() {
        let svg = session_url_svg("http://192.168.1.15:7777/s/abc123#key").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        // A QR has many modules; the path data must be present.
        assert!(svg.contains("<path"));
    }

    #[test]
    fn deterministic_for_same_input() {
        let url = "http://192.168.1.15:7777/s/token#key";
        assert_eq!(session_url_svg(url).unwrap(), session_url_svg(url).unwrap());
    }

    #[test]
    fn differs_across_urls() {
        let a = session_url_svg("http://192.168.1.15:7777/s/token1#k1").unwrap();
        let b = session_url_svg("http://192.168.1.15:7777/s/token2#k2").unwrap();
        assert_ne!(a, b);
    }
}
