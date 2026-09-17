//! The Next Rust logo, served at `/logo.svg` and used as the favicon.

use next_rust::prelude::*;

pub const SVG: &str = r##"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512" role="img" aria-label="Next Rust logo">
  <title>Next Rust</title>
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#e5e5ec"/>
      <stop offset="1" stop-color="#e0e0ef"/>
    </linearGradient>
    <linearGradient id="rust" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#ff9c2a"/>
      <stop offset="0.55" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#b3260c"/>
    </linearGradient>
    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#f25c1f"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0" stop-color="#f25c1f" stop-opacity="0.35"/>
      <stop offset="1" stop-color="#f25c1f" stop-opacity="0"/>
    </radialGradient>
    <clipPath id="inner">
      <circle cx="256" cy="256" r="126"/>
    </clipPath>
  </defs>

  <rect width="512" height="512" rx="112" fill="url(#bg)"/>
  <circle cx="256" cy="256" r="230" fill="url(#glow)"/>

  <!-- gear: the Rust half -->
  <g fill="url(#rust)">
    <g id="tooth"><rect x="238" y="70" width="36" height="72" rx="8"/></g>
    <use href="#tooth" transform="rotate(30 256 256)"/>
    <use href="#tooth" transform="rotate(60 256 256)"/>
    <use href="#tooth" transform="rotate(90 256 256)"/>
    <use href="#tooth" transform="rotate(120 256 256)"/>
    <use href="#tooth" transform="rotate(150 256 256)"/>
    <use href="#tooth" transform="rotate(180 256 256)"/>
    <use href="#tooth" transform="rotate(210 256 256)"/>
    <use href="#tooth" transform="rotate(240 256 256)"/>
    <use href="#tooth" transform="rotate(270 256 256)"/>
    <use href="#tooth" transform="rotate(300 256 256)"/>
    <use href="#tooth" transform="rotate(330 256 256)"/>
  </g>
  <circle cx="256" cy="256" r="146" fill="none" stroke="url(#rust)" stroke-width="40"/>
  <circle cx="256" cy="256" r="126" fill="#f6f6f9"/>

  <!-- the "N": the Next half, its stroke racing out of the gear -->
  <g clip-path="url(#inner)">
    <rect x="190" y="186" width="30" height="140" rx="4" fill="#f25c1f"/>
    <polygon points="190,186 222,186 350,360 318,360" fill="url(#fade)"/>
    <rect x="292" y="186" width="30" height="92" rx="4" fill="url(#fade)"/>
  </g>
</svg>
"##;

/// `GET /logo.svg`
pub async fn serve(_req: Request) -> Response {
    Response::text(SVG).with_content_type("image/svg+xml").with_cache_control("public, max-age=86400")
}
