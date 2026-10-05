# ASCII 3D Renderer in Rust

A software 3D renderer that draws animated, textured models with ASCII characters and 24-bit terminal colors. Load Wavefront OBJ models, mix colored point lights, control the camera and rotation, and resize your Windows terminal while the renderer stays centered.

## Demo video

[Watch or download the example video](https://github.com/devil12juju/ascii-3d-rust/releases/download/v0.1.0/Video.mp4).

## Download and run

Download `ascii-3d-rust-v0.1.0-windows-x64.zip` from [Releases](https://github.com/devil12juju/ascii-3d-rust/releases), extract **the entire folder**, and open `Run.cmd` for the animated torus. No Rust installation is required for the prebuilt Windows executable.

Use a Windows terminal that supports ANSI 24-bit color. Press **Q** or **Escape** to exit the animation and restore the terminal screen.

From Command Prompt, inside the extracted folder:

```cmd
Run --model models\cube.obj --center
Run --model models\textured-cube.obj --center --rotation 20,35,0
Run --help
```

You can also drag an OBJ file onto `Run.cmd`, or pass its path directly:

```cmd
Run "C:\Models\my model.obj"
```

In PowerShell, invoke the executable directly:

```powershell
.\ascii-3d-rust.exe --model .\models\textured-cube.obj --center
```

The release includes the executable, launcher, this guide, credits, license notices, an untextured cube, and a textured cube. The default torus is generated at runtime.

## Examples

### Keep a vehicle upright while rotating

```cmd
Run --model "C:\Models\vehicle.obj" --center --pitch-limit 0 --roll-limit 0
```

Pitch is rotation around X, yaw around Y, and roll around Z. The animation changes yaw and pitch; roll comes from `--rotation`. A limit of 0 locks that axis at 0 degrees. A limit of 10 clamps it between -10 and +10 degrees. Once the pitch reaches a limit, it stays at the boundary while yaw continues. There is no automatic bounce.

```cmd
Run --model models\cube.obj --center --pitch-limit 10 --roll-limit 5 --speed 0.3
```

### Adapt to window resizing

```cmd
Run --model models\cube.obj --center --margin 3 --scale 0.7
```

Automatic sizing is enabled by default on Windows. Every frame reads the visible console dimensions and recalculates the projection, viewport center, and depth buffer. One row and one column are reserved to avoid scrolling. Rendering is capped at 400 columns and 200 rows.

If the window becomes too small for your requested margins, the renderer temporarily reduces them and restores them when space becomes available. If console dimensions cannot be read, including redirected output and non-Windows platforms, it uses the configured fallback size (100 x 40 by default).

Options apply in the order you type them: `--size`, `--width`, and `--height` select fixed sizing; a later `--center` or `--auto-size` enables automatic sizing again. To keep a centered, fixed 100 x 40 image:

```cmd
Run --model models\cube.obj --center --size 100x40 --margin 3
```

### Set position, margins, and initial rotation

```cmd
Run --model models\cube.obj --size 120x45 --margins 4,2,8,3 --offset 10,-2 --static --rotation 25,40,0
Run --model models\cube.obj --position 0.5,0,1 --zoom 1.2 --speed 0.3 --fps 60
```

`--position` moves the model in 3D. `--offset` moves its projected image in character cells; positive X moves right and positive Y moves down. `--center` resets both to zero. The model is scaled, rotated around X then Y then Z, translated in 3D, and finally projected and offset on screen.

### Use white light to show texture colors

```cmd
Run --model models\textured-cube.obj --center --light "0,3,-4:255,255,255" --ambient 0.2
```

### Mix colored lights

```cmd
Run --model models\cube.obj --center --light "-3,3,-3:255,40,20" --light "3,1,-2:30,100,255" --color "220,220,220"
```

Repeat `--light` for additional point lights. Providing any lights replaces the three default red/orange, blue, and green lights. Colors use channel values from 0 to 255.

### Monochrome and custom characters

```cmd
Run --model models\cube.obj --center --monochrome
```

In PowerShell, a custom palette can be provided without batch-file percent expansion:

```powershell
.\ascii-3d-rust.exe --model .\models\cube.obj --mono --chars '.:-=+*#%@'
```

Palettes must contain 2 to 128 printable ASCII characters ordered from dark to bright. Monochrome mode removes color codes, but lights and textures still affect the chosen characters.

### Save a single plain-text frame

```cmd
Run --model models\cube.obj --center --size 80x30 --once --mono > frame.txt
Run --model models\cube.obj --frames 120
```

`--once` renders the initial orientation without animated rotation or screen-control codes. Use `--mono` for an ordinary text file. `--frames` ends after the given positive number of frames. The configured frame rate is a target, not a guarantee.

## Models, materials, and textures

The model loader accepts OBJ vertex positions, UV coordinates, negative indices, and face references in `v`, `v/vt`, `v//vn`, and `v/vt/vn` form. Triangles, quads, and convex polygons are supported; triangulate concave polygons before exporting. Imported meshes are centered using their bounding box and normalized to a radius of 1.8 before `--scale` is applied.

The renderer reads `mtllib` and `usemtl` from the OBJ. Supported MTL properties are:

| Property | Effect |
| --- | --- |
| `Kd` | Diffuse material color, RGB channels 0..1. |
| `Ks` | Specular reflection color, RGB channels 0..1. |
| `Ns` | Reflection concentration, clamped to 1..1000. |
| `map_Kd` | Diffuse image texture. |

Supported texture formats are PNG, JPEG, BMP, TGA, and PPM. UV coordinates are interpolated with perspective correction, including triangles clipped at the near plane. Textures use bilinear filtering and are converted from sRGB into linear RGB before lighting.

Keep the OBJ, MTL, and texture files together with their original relative paths:

```text
my-model/
  model.obj
  model.mtl
  textures/
    diffuse.png
```

Material library paths are relative to the OBJ folder. Texture paths are relative to the MTL folder. Paths with spaces are accepted. For `map_Kd`, `-s`, `-o`, and `-clamp` control UV scaling, offset, and repetition. Other recognized map options are consumed but do not affect the image.

If no material library is referenced, the loader looks for a same-name `.mtl` or `.mlt`. You can override it explicitly; that path is relative to your working directory:

```cmd
Run --model "C:\Models\model.obj" --mtl "C:\Models\materials.mtl" --center
```

A missing or unreadable image produces a warning and falls back to the material color. A missing automatically referenced MTL also produces a warning. A missing explicit `--mtl` is an error. Faces without UVs use their material color instead of a texture. `--no-textures` disables texture application, `--color` tints all materials, and an explicit `--shininess` overrides MTL `Ns`.

### Converting GLB or glTF models

Direct GLB/glTF loading is not implemented. Export to OBJ with a modeling tool such as Blender, and preserve the material library, UV coordinates, and image files. GLB files may contain embedded textures, so copying only the geometry can lose the original appearance. Check that the exported MTL contains `map_Kd` references to existing images; an MTL containing only gray `Kd` values cannot reproduce missing texture images. Complex shader graphs and procedural materials may require baking into diffuse images before export.

## Complete option reference

| Option | Default / accepted values | Description |
| --- | --- | --- |
| `--model PATH` | Generated torus | Load a Wavefront OBJ. |
| `--mtl PATH` | Automatic | Override material libraries. |
| `--no-textures` | Textures enabled | Use material colors without applying images. |
| `--width N` | 100, range 10..400 | Fixed output width in characters. |
| `--height N` | 40, range 5..200 | Fixed output height in characters. |
| `--size WxH` | 100x40 fallback | Set both fixed dimensions. |
| `--auto-size` | Enabled | Follow the Windows terminal size. |
| `--fixed-size` | Disabled | Keep configured dimensions. |
| `--scale N` | 1, range 0.01..10 | Change physical model size. |
| `--zoom N` | 1, range 0.01..10 | Magnify the projection without moving lights. |
| `--position X,Y,Z` | 0,0,0 | Move the model in 3D. |
| `--offset X,Y` | 0,0 | Move the image in character cells. |
| `--center` | Centered | Reset position/offset and enable auto-size. |
| `--margin N` | 0 | Equal nonnegative margins on all sides. |
| `--margins L,T,R,B` | 0,0,0,0 | Left, top, right, bottom margins. |
| `--distance N` | 5, range 0.2..100 | Camera distance from the origin. |
| `--aspect N` | 0.5, range 0.1..2 | Character width/height ratio. |
| `--rotation X,Y,Z` | 0,0,0 | Initial angles in degrees. |
| `--pitch-limit N` | Unlimited, range 0..180 | Clamp X rotation to -N..+N; 0 locks it. |
| `--roll-limit N` | Unlimited, range 0..180 | Clamp Z rotation to -N..+N; 0 locks it. |
| `--speed N` | 0.6, range -20..20 | Animated yaw speed in radians/second; pitch advances at 0.43 times this rate. |
| `--static` | Animation enabled | Set rotation speed to 0. |
| `--fps N` | 30, range 1..120 | Target frames per second. |
| `--once` | Continuous animation | Render one initial frame. |
| `--frames N` | Unlimited | Render a positive number of frames. |
| `--mono`, `--monochrome` | Color | Disable ANSI colors. |
| `--color R,G,B` | Linear RGB 0.8 per channel, approximately sRGB 231 | Tint the model; inputs use sRGB 0..255. |
| `--light X,Y,Z:R,G,B` | Three colored lights | Add a point light; repeatable. |
| `--ambient N` | 0.055, range 0..2 | Ambient light intensity. |
| `--specular N` | 0.35, range 0..2 | Reflection intensity; 0 disables it. |
| `--shininess N` | MTL `Ns`, otherwise 32; CLI range 1..256 | Override reflection concentration. |
| `--chars TEXT` | `.,:;irsXA253hMHGS#9B&@` | Dark-to-bright ASCII palette. |
| `--help`, `-h` | — | Print command-line help. |

Configured margins must leave at least two rows and columns at startup. The renderer clips model surfaces to the area inside those margins. Large scale, zoom, or offset values can move parts of a model off screen.

## How rendering works

The software rasterizer transforms model vertices, clips triangles against a near plane 0.1 units in front of the camera, and projects them onto character cells. A reciprocal-depth buffer selects the visible surface. It computes a perspective-correct world position and UV for each covered cell, samples its material texture, and evaluates ambient light, Lambert diffuse lighting, and Blinn-Phong reflections with distance attenuation.

Lighting contributions are added in linear RGB and converted to sRGB for terminal output. Weighted luminance selects an ASCII character. The camera sits at `(0,0,-distance)` and looks toward the origin: X points right, Y up, and positive Z away from the camera.

## Build from source

Install Rust with a functioning linker, then run:

```sh
cargo build --release --locked
cargo test --locked
```

On Windows with a configured MinGW toolchain, `Build.ps1` selects the installed stable GNU toolchain when `gcc` is available. Otherwise it uses your default Rust toolchain:

```powershell
.\Build.ps1
.\Build.ps1 -MingwBin "C:\Tools\mingw64\bin"
```

The binary is `target\release\ascii-3d-rust.exe` on Windows and `target/release/ascii-3d-rust` on Unix. Rust dependencies are downloaded on the first build. The published Windows executable includes the image decoder; it does not require a separate texture library installation.

The included tests cover live resize and centering, fixed dimensions, pitch/roll constraints, OBJ indices and materials, texture orientation, PNG/JPEG decoding, and paths containing spaces.

## Limitations and troubleshooting

- Windows x64 is the distributed and tested platform. The core renderer can build on other platforms, but automatic terminal sizing and Q/Escape handling currently use Windows APIs. On other platforms, use fixed sizing and Ctrl+C.
- Normals are calculated per triangle. OBJ normals, smooth shading, shadows, transparency/alpha, bump maps, and full physically based materials are not implemented.
- Detailed textures are reduced to the terminal's character grid. Increase the window size or use a larger fixed output size for more detail.
- If colors look different, try one white light and remove `--monochrome`.
- If textures are missing, read the warnings and check the MTL `map_Kd` paths, image files, and OBJ UVs.
- If the model is stretched, adjust `--aspect` to match your terminal font.
- If the model disappears, try `--center --scale 1 --zoom 1 --distance 5`.
- For fixed output, keep the terminal at least one row and column larger than the configured image to avoid scrolling.

## Credits and license

Created by **[devil12juju](https://github.com/devil12juju)** (Juan), with development assistance from **OpenAI Codex**. The cube, generated torus, and checker texture are project examples created for this renderer.

Texture decoding uses the Rust **[image](https://github.com/image-rs/image)** library and its contributors' work. See [CREDITS.md](CREDITS.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for dependency acknowledgments. Third-party license texts are included in `third-party-licenses/` in the Windows release and in `third-party-licenses.zip` in the source repository.

The renderer and its project examples are available under the [MIT License](LICENSE). User-supplied models and textures keep their own licenses and are not included in the public release.
