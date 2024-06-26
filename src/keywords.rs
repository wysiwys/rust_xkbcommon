// This file is autogenerated by codegen/src/keywords.rs
/*
 * Copyright © 2024 wysiwys
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice (including the next
 * paragraph) shall be included in all copies or substantial portions of the
 * Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 *
 */

use crate::lexer::Token;
use unicase::*;

pub(crate) static KEYWORDS: phf::OrderedMap<UniCase<&'static str>, Token> = ::phf::OrderedMap {
    key: 7485420634051515786,
    disps: &[
        (5, 33),
        (5, 1),
        (0, 29),
        (1, 5),
        (1, 24),
        (2, 29),
        (39, 31),
        (11, 0),
        (2, 28),
    ],
    idxs: &[
        13, 32, 37, 7, 29, 4, 6, 11, 39, 25, 18, 28, 19, 20, 0, 27, 41, 30, 21, 26, 31, 9, 33, 5,
        24, 38, 15, 36, 17, 14, 42, 1, 12, 3, 40, 22, 35, 2, 10, 8, 43, 16, 34, 44, 23,
    ],
    entries: &[
        (UniCase::ascii("action"), Token::ActionTok),
        (UniCase::ascii("alias"), Token::Alias),
        (UniCase::ascii("alphanumeric_keys"), Token::AlphanumericKeys),
        (UniCase::ascii("alternate_group"), Token::AlternateGroup),
        (UniCase::ascii("alternate"), Token::Alternate),
        (UniCase::ascii("augment"), Token::Augment),
        (UniCase::ascii("default"), Token::Default),
        (UniCase::ascii("function_keys"), Token::FunctionKeys),
        (UniCase::ascii("group"), Token::Group),
        (UniCase::ascii("hidden"), Token::Hidden),
        (UniCase::ascii("include"), Token::Include),
        (UniCase::ascii("indicator"), Token::Indicator),
        (UniCase::ascii("interpret"), Token::Interpret),
        (UniCase::ascii("keypad_keys"), Token::KeypadKeys),
        (UniCase::ascii("key"), Token::Key),
        (UniCase::ascii("keys"), Token::Keys),
        (UniCase::ascii("logo"), Token::Logo),
        (UniCase::ascii("modifier_keys"), Token::ModifierKeys),
        (UniCase::ascii("modifier_map"), Token::ModifierMap),
        (UniCase::ascii("mod_map"), Token::ModifierMap),
        (UniCase::ascii("modmap"), Token::ModifierMap),
        (UniCase::ascii("outline"), Token::Outline),
        (UniCase::ascii("overlay"), Token::Overlay),
        (UniCase::ascii("override"), Token::Override),
        (UniCase::ascii("partial"), Token::Partial),
        (UniCase::ascii("replace"), Token::Replace),
        (UniCase::ascii("row"), Token::Row),
        (UniCase::ascii("section"), Token::Section),
        (UniCase::ascii("shape"), Token::Shape),
        (UniCase::ascii("solid"), Token::Solid),
        (UniCase::ascii("text"), Token::Text),
        (UniCase::ascii("type"), Token::Type),
        (UniCase::ascii("virtual_modifiers"), Token::VirtualMods),
        (UniCase::ascii("virtual"), Token::Virtual),
        (UniCase::ascii("xkb_compatibility_map"), Token::XkbCompatmap),
        (UniCase::ascii("xkb_compatibility"), Token::XkbCompatmap),
        (UniCase::ascii("xkb_compat_map"), Token::XkbCompatmap),
        (UniCase::ascii("xkb_compat"), Token::XkbCompatmap),
        (UniCase::ascii("xkb_geometry"), Token::XkbGeometry),
        (UniCase::ascii("xkb_keycodes"), Token::XkbKeycodes),
        (UniCase::ascii("xkb_keymap"), Token::XkbKeymap),
        (UniCase::ascii("xkb_layout"), Token::XkbLayout),
        (UniCase::ascii("xkb_semantics"), Token::XkbSemantics),
        (UniCase::ascii("xkb_symbols"), Token::XkbSymbols),
        (UniCase::ascii("xkb_types"), Token::XkbTypes),
    ],
};
