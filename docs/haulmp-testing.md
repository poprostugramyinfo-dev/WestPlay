# HaulMP map layer — Windows test notes

This is an experimental, opt-in layer for the existing in-cab map. It adds HaulMP player dots to WestPlay's
game-derived roads; it does not open another window or replace the map, route guidance, YouTube or Spotify.

## Data source

The public live-map endpoint is `https://map.haulmp.com/api/live`. A read on 10 November 2026 returned HTTP 200
with `online`, `players`, and ETS2 world coordinates (`x`, `z`) and heading (`h`). The live map's own page polls
this endpoint and renders players from those coordinates. Its response has no `Access-Control-Allow-Origin`
header, so the cabin WebView cannot read it directly. WestPlay's Windows host fetches it in a background thread
only while the HaulMP button is on, then sends sanitized coordinates to the in-cab page over its existing
WebView bridge.

The layer starts off. While enabled, it reads the Europe 1 feed every two seconds, drops invalid/out-of-range
coordinates, and draws only players inside the current map view. Player names and account data are not copied
into WestPlay. The public endpoint returns server player positions; it does not receive the local truck's
coordinates from this request. Switching to HaulMP's other map servers is not wired up yet.

## Build on Windows

Use the Windows requirements in the main README: stable Rust, Node.js 24, and WebView2. From the repository root:

```powershell
npm install
npm run build --prefix ui
cargo check --workspace
cargo test --workspace
npm test --prefix ui
cargo build --release -p westplay-plugin
./tools/verify-plugin.ps1 -Path ./target/release/westplay.dll
```

The hosted Windows check skips two existing tests that depend on a physical GPU and a non-elevated process;
the hosted runner exposes a virtual adapter and runs its test process elevated. All other workspace tests run,
including the HaulMP feed tests. Run the full commands above on a normal Windows PC as well.

The GitHub Actions run publishes `westplay.dll` as a 90-day downloadable artifact. The Windows check verifies
that it is an x64 PE DLL, contains the ETS2 SCS telemetry entry points, and can be mapped by the Windows DLL
loader with its imports resolved. This is a loader/format check, not an in-game check; do not treat HaulMP
integration as tested until the in-game steps below pass.

Download the ZIP from the workflow run's **Artifacts** section, unpack `westplay.dll`, and place it in the
ETS2 `bin/win_x64/plugins` folder as documented by the WestPlay setup. The artifact contains the game plugin
only; it does not bundle the WestPlay companion app or its game-specific setup files.

For the full companion app, use `npm run dev` as documented in the main README. Cargo may need an internet
connection the first time to download the new `reqwest` dependency.

## In-game check

1. Build and start the modified companion, then launch ETS2 through HaulMP and join Europe 1.
2. Open WestPlay on a supported in-cab screen and tap **HaulMP** in the map's upper-right corner.
3. Check that nearby public map positions appear on WestPlay's roads and move after successive feed updates.
4. Turn the layer off and confirm its dots disappear; check route guidance, YouTube and Spotify still work.
5. Repeat with the map panned away from the truck and after temporarily disconnecting the network.

This checkout has not been run in a live HaulMP/ETS2 session. Treat the feature as experimental until those
steps pass on the target Windows installation.
