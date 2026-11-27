Example of a window with 10 bits per color channel (`NativeOptions::prefer_10_bit_color`), to check
whether your setup shows 10-bit color.

Each gradient is drawn twice: in the upper row as 8 flat steps, in the lower row smoothly. If the
lower row shows the same 8 wide steps, something is 8-bit: the window, the GPU output or the
monitor. If every step splits into about 4 narrower ones, your monitor shows 10-bit color.

```sh
cargo run -p ten_bit_color            # glow
cargo run -p ten_bit_color -- --wgpu  # wgpu
cargo run -p ten_bit_color -- --8-bit # an 8-bit window, to compare
```
