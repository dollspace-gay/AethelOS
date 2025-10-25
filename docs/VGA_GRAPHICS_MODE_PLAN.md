# VGA Graphics Mode Implementation Plan for AethelOS

## Overview
Transform AethelOS from VGA text mode to graphics mode, enabling full Unicode support (including ◈) and setting the foundation for a GUI-capable operating system.

## Architecture Decision: Dual-Mode Support

**Keep both text mode AND graphics mode:**
- Text mode: Fast, simple, fallback
- Graphics mode: Unicode, beautiful, future-proof
- User can choose at boot or switch dynamically

## Phase 1: Basic Graphics Infrastructure (1-2 days)

### 1.1 Create Graphics Module Structure
**New files:**
- `heartwood/src/weave_canvas/mod.rs` - Main graphics module ("Weave Canvas" - thematic!)
- `heartwood/src/weave_canvas/vga.rs` - VGA hardware control
- `heartwood/src/weave_canvas/framebuffer.rs` - Framebuffer abstraction
- `heartwood/src/weave_canvas/mode.rs` - Display mode management

### 1.2 Implement VGA Mode Switching
- Mode 0x13 (320x200, 256 colors) - simplest to start
- Save/restore previous mode
- Detect available modes
- Mode enumeration and selection

**Code to write:**
```rust
pub enum DisplayMode {
    Text80x25,           // Current mode
    Graphics320x200,     // Mode 0x13
    Graphics640x480,     // Future: Mode 0x12
}

pub fn set_mode(mode: DisplayMode) -> Result<(), GraphicsError>
pub fn get_current_mode() -> DisplayMode
```

### 1.3 Framebuffer Access
- Map VGA memory (0xA0000) safely
- Implement pixel drawing primitives
- Color palette management

**Primitives to implement:**
```rust
fn put_pixel(x: u16, y: u16, color: u8)
fn fill_rect(x: u16, y: u16, w: u16, h: u16, color: u8)
fn clear_screen(color: u8)
fn copy_region(src_x, src_y, dst_x, dst_y, w, h) // For scrolling
```

**Estimated effort:** 8-12 hours

---

## Phase 2: Font Rendering System (2-3 days)

### 2.1 Font Format - PSF (PC Screen Font)
**Decision:** Use PSF2 format ✅

**Why PSF:**
- Standard Linux console font format
- Built-in Unicode mapping table
- Well-documented specification
- Many existing fonts available
- Supports custom glyphs (including ◈!)

**Implementation approach:**
1. Embed a PSF2 font file in the kernel binary
2. Parse PSF2 header at runtime
3. Load glyph bitmaps and Unicode table
4. Map UTF-8 characters → glyph indices

**Recommended starter font:**
- Terminus font (ter-u16n.psf or ter-u16b.psf)
- Clear, readable 8×16 bitmap font
- Excellent Unicode coverage
- ~50KB size

### 2.2 Create Rune Renderer (Font Module)
**New files:**
- `heartwood/src/weave_canvas/runes/mod.rs` - Font system ("Runes" - thematic!)
- `heartwood/src/weave_canvas/runes/bitmap.rs` - Bitmap fonts
- `heartwood/src/weave_canvas/runes/psf.rs` - PSF loader (Phase 2.5)

**Core functionality:**
```rust
pub struct RuneRenderer {
    font_data: &'static [[u8; 16]; 256],  // 8x16 bitmaps
    char_width: u8,
    char_height: u8,
}

impl RuneRenderer {
    pub fn draw_char(&self, ch: char, x: u16, y: u16, fg: u8, bg: u8)
    pub fn draw_string(&self, s: &str, x: u16, y: u16, fg: u8, bg: u8)
    pub fn measure_string(&self, s: &str) -> u16  // Width in pixels
}
```

### 2.3 Design Custom ◈ Glyph
- Create 8x16 bitmap for ◈
- Add to font data
- Test rendering

### 2.4 UTF-8 Support
- Map Unicode codepoints to glyph indices
- Handle multi-byte UTF-8 sequences
- Fallback character for unmapped glyphs

**Estimated effort:** 12-16 hours

---

## Phase 3: Terminal Emulation - "The Scribe" (3-4 days)

### 3.1 Create Terminal Abstraction
**New file:** `heartwood/src/weave_canvas/scribe.rs` ("The Scribe" writes to the canvas)

**Core structure:**
```rust
pub struct Scribe {
    cursor_x: u16,        // In characters
    cursor_y: u16,        // In characters
    cols: u16,            // Screen width in chars
    rows: u16,            // Screen height in chars

    fg_color: u8,         // Foreground color
    bg_color: u8,         // Background color

    framebuffer: &'static mut [u8],
    rune_renderer: RuneRenderer,

    // For optimization
    dirty: bool,
    needs_scroll: bool,
}
```

### 3.2 Implement Core Terminal Operations
```rust
impl Scribe {
    pub fn write_char(&mut self, ch: char)
    pub fn write_str(&mut self, s: &str)

    fn newline(&mut self)
    fn carriage_return(&mut self)
    fn backspace(&mut self)
    fn tab(&mut self)

    fn scroll_up(&mut self)  // Most complex operation!
    fn clear_line(&mut self, line: u16)
    fn clear_screen(&mut self)

    pub fn set_cursor(&mut self, x: u16, y: u16)
    pub fn get_cursor(&self) -> (u16, u16)
}
```

### 3.3 Handle Special Characters
- `\n` - Newline
- `\r` - Carriage return
- `\x08` - Backspace (erase previous char)
- `\t` - Tab (8-space alignment)
- ANSI escape codes (future: colors, cursor movement)

### 3.4 Scrolling Implementation
**Challenge:** Scrolling is expensive (must copy ~58KB)

**Optimization strategies:**
1. **Naive approach:** Copy entire framebuffer up
   ```rust
   fn scroll_up_naive(&mut self) {
       let line_bytes = self.cols * CHAR_HEIGHT * SCREEN_WIDTH;
       unsafe {
           core::ptr::copy(
               self.framebuffer.as_ptr().add(line_bytes),
               self.framebuffer.as_mut_ptr(),
               (ROWS - 1) * line_bytes,
           );
       }
       // Clear last line
       self.clear_line(self.rows - 1);
   }
   ```

2. **Optimization:** Only redraw changed text (Phase 5)

**Estimated effort:** 20-24 hours

---

## Phase 4: Integration with Existing Systems (2-3 days)

### 4.1 Create Display Abstraction Layer
**New file:** `heartwood/src/display.rs`

```rust
pub trait Display {
    fn write_str(&mut self, s: &str);
    fn write_char(&mut self, ch: char);
    fn clear(&mut self);
    fn set_color(&mut self, fg: Color, bg: Color);
}

// Implement for both:
impl Display for VgaTextWriter { ... }
impl Display for Scribe { ... }
```

### 4.2 Modify Print Macros
Update `print!` and `println!` to work with either mode:

```rust
static DISPLAY_MODE: AtomicU8 = AtomicU8::new(DisplayMode::Text as u8);
static SCRIBE: Once<Mutex<Scribe>> = Once::new();

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        match DISPLAY_MODE.load(Ordering::Relaxed) {
            0 => $crate::vga_buffer::_print(...),
            1 => $crate::weave_canvas::_print(...),
            _ => {}
        }
    });
}
```

### 4.3 Update Eldarin Shell
- Ensure all commands work in graphics mode
- Test command history navigation
- Test backspace
- Verify visual elements (progress bars, etc.)

### 4.4 Add Mode Switching Command
```rust
fn cmd_display_mode(args: &str) {
    match args {
        "text" => switch_to_text_mode(),
        "graphics" => switch_to_graphics_mode(),
        "status" => show_current_mode(),
    }
}
```

**Estimated effort:** 12-16 hours

---

## Phase 5: Optimization & Polish (2-3 days)

### 5.1 Double Buffering
**Problem:** Flickering during redraws

**Solution:**
- Allocate second framebuffer in RAM
- Draw to back buffer
- Memcpy to VGA memory in one operation

```rust
struct DoubleBuffer {
    front: &'static mut [u8],  // VGA memory
    back: Vec<u8>,             // RAM buffer
}

impl DoubleBuffer {
    fn swap(&mut self) {
        self.front.copy_from_slice(&self.back);
    }
}
```

### 5.2 Dirty Rectangle Tracking
**Problem:** Redrawing entire screen is slow

**Solution:**
- Track which regions changed
- Only redraw dirty regions

```rust
struct DirtyRegion {
    x: u16, y: u16,
    width: u16, height: u16,
}

fn mark_dirty(&mut self, region: DirtyRegion)
fn redraw_dirty_regions(&mut self)
```

### 5.3 Scrolling Optimization
**Option 1:** Ring buffer for lines
- Keep text in RAM
- Only redraw visible lines after scroll
- 10-100x faster

**Option 2:** Hardware scrolling
- Some VGA modes support hardware scrolling
- Research feasibility

### 5.4 Color Scheme
Design AethelOS color palette:
- Background: Dark blue/purple (symbiotic theme)
- Text: Light cyan/white
- Highlights: Gold/amber
- Errors: Soft red
- Success: Soft green

**Estimated effort:** 16-20 hours

---

## Phase 6: Advanced Features (Future)

### 6.1 Higher Resolutions
- Mode 0x12: 640x480, 16 colors
- VESA modes: 800x600, 1024x768 (requires VESA BIOS Extensions)
- Framebuffer support (UEFI GOP)

### 6.2 Better Font Rendering
- Anti-aliased fonts
- Multiple font sizes
- Bold/italic variants
- Font loading from filesystem

### 6.3 Full Unicode Support
- Unicode normalization
- Combining characters
- Right-to-left text
- CJK characters (complex!)

### 6.4 Graphics Primitives
- Line drawing (Bresenham)
- Circle/ellipse
- Bezier curves
- Image loading (BMP, PPM)

### 6.5 Window System Foundation
- Multiple overlapping windows
- Z-order management
- Event system
- Widget toolkit

---

## File Structure

```
heartwood/src/
├── weave_canvas/          # NEW: Graphics subsystem
│   ├── mod.rs            # Main module, mode management
│   ├── vga.rs            # VGA hardware control
│   ├── framebuffer.rs    # Framebuffer abstraction
│   ├── mode.rs           # Display mode enum/switching
│   ├── scribe.rs         # Terminal emulation
│   └── runes/            # Font rendering
│       ├── mod.rs
│       ├── bitmap.rs     # Hardcoded font data
│       └── psf.rs        # PSF loader (later)
├── display.rs            # NEW: Display trait abstraction
├── vga_buffer.rs         # MODIFY: Implement Display trait
├── eldarin.rs            # MODIFY: Add display-mode command
└── lib.rs                # MODIFY: Initialize graphics
```

---

## Testing Strategy

### Unit Tests
- Pixel drawing functions
- Character rendering
- UTF-8 decoding
- Color conversion

### Integration Tests
- Mode switching
- Text rendering
- Scrolling
- Command execution

### Manual Tests
- Boot in graphics mode
- Run all Eldarin commands
- Test command history
- Test backspace
- Verify ◈ displays correctly
- Test long text output
- Test scrolling performance

---

## Risk Mitigation

### Risk 1: Performance Issues
**Mitigation:**
- Implement optimizations early
- Profile hotspots
- Keep text mode as fallback

### Risk 2: Complexity Explosion
**Mitigation:**
- Implement in small, testable phases
- Don't over-engineer early
- Get basic version working first

### Risk 3: VGA Hardware Quirks
**Mitigation:**
- Test on QEMU first
- Test on real hardware
- Research known issues
- Keep good documentation

### Risk 4: Breaking Existing Functionality
**Mitigation:**
- Keep text mode working
- Gradual migration
- Extensive testing

---

## Timeline Estimate

| Phase | Time | Cumulative |
|-------|------|------------|
| Phase 1: Graphics Infrastructure | 8-12 hours | 1.5 days |
| Phase 2: Font Rendering | 12-16 hours | 3.5 days |
| Phase 3: Terminal Emulation | 20-24 hours | 6.5 days |
| Phase 4: Integration | 12-16 hours | 8.5 days |
| Phase 5: Optimization | 16-20 hours | 11 days |
| **Total (MVP)** | **68-88 hours** | **11-14 days** |

*Assumes focused work; real-time may be 2-3 weeks*

---

## Success Criteria

**Phase 1-3 (MVP):**
- ✅ Can boot into graphics mode
- ✅ Eldarin shell works
- ✅ ◈ displays correctly
- ✅ All commands functional
- ✅ Command history works
- ✅ Acceptable performance

**Phase 4-5 (Polish):**
- ✅ No flickering
- ✅ Smooth scrolling
- ✅ Beautiful color scheme
- ✅ Can switch between text/graphics mode

**Phase 6 (Future):**
- ✅ Higher resolutions
- ✅ Full Unicode
- ✅ Foundation for GUI

---

## Design Decisions

### Resolved Preferences

1. **Font choice:** ✅ Use existing PSF (PC Screen Font) format
   - Standard Linux console font format
   - Supports Unicode mapping out of the box
   - Well-documented and battle-tested
   - Can start with a standard PSF2 font file

2. **Default mode:** ✅ Boot into graphics mode by default
   - Graphics mode enabled at startup
   - Text mode available as fallback (switchable via command or if graphics fails)
   - Provides best user experience immediately

3. **Compatibility:** ✅ QEMU-only initially
   - Focus on getting it working perfectly in QEMU first
   - Real hardware support comes later
   - Simplifies testing and debugging

4. **Memory usage:** ✅ 64KB for back buffer is acceptable
   - Mode 0x13 (320x200) requires exactly 64,000 bytes
   - Acceptable memory overhead for smooth rendering
   - Eliminates flickering

### Open Questions

1. **Color palette:** What specific colors best represent AethelOS's symbiotic aesthetic?
   - Suggestion: Dark blue/purple background, cyan/white text, gold/amber highlights
   - Needs testing and refinement

---

## Implementation Roadmap

### Initialization Sequence (with defaults applied)

```rust
// At boot (in main.rs or initialization)
fn init_display() -> Result<(), DisplayError> {
    // Try to initialize graphics mode
    match weave_canvas::init_graphics_mode() {
        Ok(_) => {
            println!("✓ Graphics mode initialized (320×200)");
            println!("  Type 'display-mode text' to switch to text mode");
        }
        Err(e) => {
            println!("⚠ Graphics mode failed, using text mode fallback");
            println!("  Error: {:?}", e);
            // Text mode already active, continue
        }
    }
    Ok(())
}
```

### Next Steps

When ready to begin implementation:
1. **Phase 1** - Basic Graphics Infrastructure
   - Set up VGA mode 0x13 (320×200, 256 colors)
   - Test pixel drawing in QEMU

2. **Phase 2** - Font Rendering
   - Embed Terminus PSF2 font (~50KB)
   - Implement PSF2 parser
   - Test rendering ASCII and ◈

3. **Phase 3** - Terminal Emulation
   - Build "The Scribe" terminal emulator
   - Implement scrolling
   - Test with Eldarin shell

4. **Phase 4** - Integration
   - Make graphics mode the default
   - Add text mode fallback
   - Add display-mode switching command

5. **Phase 5** - Polish
   - Implement double buffering
   - Choose color palette
   - Optimize scrolling

### Development Guidelines
- Test each phase thoroughly in QEMU before proceeding
- Keep text mode functional as fallback at all times
- Document all VGA register interactions
- Commit frequently with descriptive messages
- Measure performance at each stage
