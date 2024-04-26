// Includes descriptions from xkbcommon.h
//!
//! A port of `libxkbcommon` version `1.7.0` in safe Rust.
//!
//! This crate is intended for use within a Wayland client written in Rust. It provides `Send + Sync` implementations of [Keymap] and [State].
//! # Example
//!
//! To set up the keymap/state on the Wayland client side:
//! ```rust
//! use rust_xkbcommon::*;
//!
//! let keymap = Keymap::new_from_string(
//!     Context::new(0).unwrap(),
//!     string, /* Read from the OwnedFd provided by the Wayland compositor */
//!     KeymapFormat::TextV1,
//!     0).unwrap();
//!
//! let mut state = State::new(keymap);
//!
//! ```
//!
//! To get syms and update the state on the client side:
//!
//!
//! ```rust
//! // Get syms before updating state
//! let sym = state.key_get_one_sym(keycode)?;
//!
//! // Update state with the parameters provided by the wl_keyboard::Event::Modifiers{..} event
//! state.update_mask(
//!     mods_depressed, mods_latched, mods_locked,
//!     0, 0, group as usize);
//! ```
//!
//! For more information on using [State::update_mask()] in a Wayland client, please see <https://wayland-book.com/seat/keyboard.html>.
//!
//! # Server state and client state
//! The `xkb_state` API is used by two distinct actors in most window-system architectures:
//! 1. A *server* - for example, a Wayland compositor, and X11 server, or an evdev listener.
//!
//! Servers maintain the XKB state for a device according to input events from the device, such as
//! key presses and releases, and out-of-band events from the user, like UI layout switchers.
//!
//! 2. A *client* - for example, a Wayland client, an X11 client.
//!
//! Clients do not listen to input from the device; instead, whenever the server state changes, the
//! server state serializes the state and notifies the clients that the state has changed; the
//! clients then update the state from the serialization.
//!
//! Some entry points in the `xkb_state` API are only meant for servers and some are only meant for
//! clients, and the two should generally not be mixed.
//!
//!
//! # Environment variables
//!
//! The user may set some environment variables which affect the library:
//!
//! `XKB_CONFIG_ROOT`, `XKB_CONFIG_EXTRA_PATH`, `XDG_CONFIG_DIR`, `HOME` - see [include-path].
//! `XKB_DEFAULT_RULES`, `XKB_DEFAULT_MODEL`, `XKB_DEFAULT_LAYOUT`, `XKB_DEFAULT_VARIANT`,
//! `XKB_DEFAULT_OPTIONS` - see [xkb_keymap::RuleNames].
//!
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(clippy::module_inception)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::absurd_extreme_comparisons)]
#![crate_name = "rust_xkbcommon"]
#![forbid(unsafe_code)]
mod keysyms_utf;
// generated in project 26
// TODO: migrate over to build.rs
mod keysyms_generated_phf;

mod keysyms;

mod atom;
mod context;
mod errors;
mod keymap;
mod message_codes;
mod state;

mod rust_xkbcommon;
mod xkbcomp;

mod parser_utils;
mod utils;

mod text;
mod utf8;

mod config;

pub mod error {

    pub use super::errors::{context, keymap, state};
}
pub mod xkb_context {
    /// Opaque top-level library context object.
    ///
    /// The context contains various general library data and state, like
    /// include paths.
    ///
    /// Objects are created in a specific context, and multiple contexts may coexist simultaneously.
    /// Objects from different contexts are completely separated and do not share any memory or state.
    ///
    pub use super::context::Context;

    pub use super::rust_xkbcommon::ContextFlags;
}
pub use xkb_context::Context;

pub mod xkb_keymap {
    /// Opaque compiled keymap object.
    ///
    /// The keymap object holds all of the static keyboard information obtained from compiling XKB
    /// files.
    ///
    /// A keymap is immutable after it is created.
    /// If you need to change it, you must create a new one.
    pub use super::keymap::Keymap;

    pub use super::rust_xkbcommon::CompileFlags;

    /// Names to compile a keymap with, also known as RMLVO.
    ///
    /// The names are the common configuration values by which a user picks a keymap.
    ///
    /// If a `None` is passed to [Keymap::new_from_names()], then each field is taken to be `None`.
    /// You should prefer passing `None` instead of choosing your own defaults.
    pub use super::rust_xkbcommon::RuleNames;

    pub use super::rust_xkbcommon::KeymapFormat;
}
pub use xkb_keymap::Keymap;
pub use xkb_keymap::KeymapFormat;

pub mod xkb_state {

    /// Opaque keyboard state object.
    ///
    /// State objects contain the active state of a keyboard
    /// (or keyboards), such
    /// as the currently effective layout and the active modifiers.
    /// It acts as a simple state machine, wherein key presses
    /// and releases are the input, and key symbols (keysyms) are the output.
    pub use super::state::State;

    pub use super::rust_xkbcommon::KeyDirection;
    /// Index of a keyboard layout.
    ///
    /// The layout index is a state component which determines which <em>keyboard layout</em> is
    /// active. These may be different alphabets, different key arrangements, etc.
    ///
    /// Layout indices are consecutive. The first layout has index 0.
    ///
    /// Each layout is not required to have a name, and the names are not guaranteed to be unique
    /// (though they are usually provided and unique).
    /// Therefore, it is not safe to use the name as a unique identifier for a layout.
    /// Layout names are case-sensitive.
    ///
    /// Layout names are specified in the layout's definition,
    /// for example "English (US)". These are different from the
    /// (conventionally) short names which are used to locate the layout,
    /// for example "us" or "us(intl)". These names are not present
    /// in a compiled keymap.
    ///
    /// If the user selects layouts from a list generated from the XKB registry (using libxkbregistry
    /// or directly), it is recommended to store it along with the keymap.
    ///
    /// Layouts are also called "groups" by XKB.
    ///
    pub use super::rust_xkbcommon::LayoutIndex;

    /// A mask of layout indices.
    pub use super::rust_xkbcommon::LayoutMask;

    /// Index of a shift level.
    ///
    pub use super::rust_xkbcommon::LevelIndex;

    /// Index of a modifier.
    ///
    pub use super::rust_xkbcommon::ModIndex;

    /// A mask of modifier indices.
    ///
    pub use super::rust_xkbcommon::ModMask;

    /// Index of a keyboard LED.
    ///
    pub use super::rust_xkbcommon::LedIndex;

    /// A mask of LED indices.
    ///
    pub use super::rust_xkbcommon::LedMask;

    pub use super::rust_xkbcommon::names::*;
    /// Consumed modifiers mode.
    ///
    /// There are several possible methods for deciding which modifiers are consumed and which are not,
    /// each applicable for different systems or situations. The mode selects the method to use.
    ///
    /// Keep in mind that in all methods the keymap may decide to "preserve" a modifier, meaning it is
    /// not reported as consumed even if it would have otherwise.
    pub use super::rust_xkbcommon::ConsumedMode;

    pub use super::rust_xkbcommon::StateComponent;

    pub use super::keymap::XKB_MAX_GROUPS;
}
pub use xkb_state::State;

/// A number used to represent a physical key on a keyboard.
///
/// A standard PC-compatible keyboard might have 102 keys.
/// An appropriate keymap would assign each of them a keycode,
/// by which the user should refer to the key throughout the library.
///
/// Historically, the X11 protocol, and consequentially the XKB protocol,
/// assign only 8 bits for keycodes. This limits the number of different keys
/// that can be used simultaneously in a single keymap to 256
/// (disregarding other limitations). This library does not share this limit;
/// keycodes beyond 255 ('extended keycodes') are not treated specially.
/// Keymaps and applications which are compatible with X11
/// should not use these keycodes.
///
/// The values of specific keycodes are determined by the keymap and the underlying input system.
/// For example, with an X11-compatible keymap
/// and Linux evdev scan codes (see [evdev::Key](https://docs.rs/evdev/latest/evdev/struct.Key.html)), a fixed offset is used:
///
/// ```
/// use evdev::Key;
/// let keycode_A = Keycode::new(Key::KEY_A + 8);
/// ```
///
/// The keymap defines a canonical name for each key, plus possible aliases.
/// Historically, the XKB protocol restricts these names to at most 4 (ASCII) characters,
/// but this library does not share this limit.
pub mod keycode {
    pub use super::rust_xkbcommon::RawKeycode;

    /// A wrapper struct for [RawKeycode].
    ///
    #[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
    pub struct Keycode(pub RawKeycode);
}

// parser generated in build.rs
mod lexer;
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub(crate) parser);

// keywords list generated in build.rs
mod keywords;

/// Re-export of [`xkeysym`]
pub mod keysym {
    /// Re-export of [`xkeysym::NO_SYMBOL`]:
    pub use xkeysym::NO_SYMBOL;

    /// Re-export of [`xkeysym::Keysym`]
    ///
    /// A keycode is a number used to represent the symbols generated from a key on a keyboard.
    ///
    /// A key, represented by a keycode, may generate different symbols
    /// according to keyboard state. For example, on a QWERTY keyboard,
    /// pressing the key labeled \<A\> generates the symbol 'a'. If the shift key is held, it generates
    /// the symbol  ‘α’.  And so on.
    ///
    /// Each such symbol is represented by a *keysym* (short for "key symbol").
    /// Note that keysyms are somewhat more general, in that they can also represent
    /// some "function", such as "Left" or "Right" for the arrow keys.
    /// For more information, see Appendix A ["KEYSYM
    /// Encoding"](https://www.x.org/releases/X11R7.7/doc/xproto/x11protocol.html#keysym_encoding) of the X Window System
    /// Protocol.
    ///
    /// Keysym names are case-sensitive.
    ///
    pub use xkeysym::Keysym;

    /// Get the name of a keysym.
    ///
    /// For a description of how keysyms are named, see [Keysym].
    ///
    pub use super::keysyms::keysym_get_name;

    pub use super::keysyms::keysym_is_keypad;

    pub use super::keysyms::keysym_is_lower;
    pub use super::keysyms::keysym_is_modifier;
    pub use super::keysyms::keysym_is_upper;
    pub use super::keysyms::keysym_to_lower;
    pub use super::keysyms::keysym_to_upper;

    /// The flags for [keysym_from_name()].
    pub use super::rust_xkbcommon::KeysymFlags;

    /// Get a keysym from its name.
    ///
    /// # Arguments
    /// * `name`: The name of a keysym. See remarks in [keysym_get_name()];
    /// this function will accept any name returned by that function.
    /// * `flags`: A set of flags controlling how the search is done. If invalid flags are passed, this
    /// will fail with `None`.
    ///
    /// If you use the [KeysymFlags::CASE_INSENSITIVE] flag and two keysym names differ only by case,
    /// then the lower-case keysym is returned. For instance, for `KEY_a` and `KEY_A`, this function
    /// would return `KEY_a` for the case-insensitive search. If this functionality is needed, it is
    /// recommended to first call this function without this flag; and if that fails, only then to try
    /// with this flag, while possibly warning the user he had misspelled the name, and might get wrong
    /// results.
    pub use super::keysyms::keysym_from_name;
    /// Maximum keysym value
    ///
    pub use super::rust_xkbcommon::XKB_KEYSYM_MAX;
}

#[cfg(test)]
pub(crate) mod test;

#[cfg(test)]
macro_rules! log_init {
    () => {
        use simplelog::*;

        TermLogger::init(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .unwrap()
    };
}
#[cfg(test)]
pub(crate) use log_init;