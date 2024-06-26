// based on types.c
/************************************************************
 * Copyright (c) 1994 by Silicon Graphics Computer Systems, Inc.
 *
 * Permission to use, copy, modify, and distribute this
 * software and its documentation for any purpose and without
 * fee is hereby granted, provided that the above copyright
 * notice appear in all copies and that both that copyright
 * notice and this permission notice appear in supporting
 * documentation, and that the name of Silicon Graphics not be
 * used in advertising or publicity pertaining to distribution
 * of the software without specific prior written permission.
 * Silicon Graphics makes no representation about the suitability
 * of this software for any purpose. It is provided "as is"
 * without any express or implied warranty.
 *
 * SILICON GRAPHICS DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
 * SOFTWARE, INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE. IN NO EVENT SHALL SILICON
 * GRAPHICS BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL
 * DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE,
 * DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE
 * OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION  WITH
 * THE USE OR PERFORMANCE OF THIS SOFTWARE.
 *
 ********************************************************/
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

use super::ast::*;
use crate::atom::Atom;
use crate::context::Context;
use crate::keymap::{KeyType, KeyTypeEntry, KeymapBuilder, ModSet, ModType, Mods};
use crate::rust_xkbcommon::*;

use crate::errors::*;

use std::collections::BTreeMap;

bitflags::bitflags! {
    #[derive(Copy,Clone)]
    pub(crate) struct TypeField: u8 {
        const Mask = 1 << 0;
        const Map = 1 << 1;
        const Preserve = 1 << 2;
        const LevelName = 1 << 3;

    }
}

struct KeyTypeInfo {
    defined: TypeField,
    merge: MergeMode,
    name: Atom,
    mods: ModMask,
    num_levels: LevelIndex,
    entries: Vec<KeyTypeEntry>,
    level_names: BTreeMap<usize, Atom>,
}

pub(super) struct KeyTypesInfo {
    name: Option<String>,
    errors: Vec<CompileTypesError>,
    unrecoverable_error: Option<CompileTypesError>,
    include_depth: u32, //unsigned int
    types: Vec<KeyTypeInfo>,
    pub(super) mods: ModSet,
}

impl Context {
    fn map_entry_txt(&self, info: &KeyTypesInfo, entry: &KeyTypeEntry) -> String {
        self.mod_mask_text(&info.mods, entry.mods.mods)
    }

    fn type_txt<'a>(&'a self, _type: &KeyTypeInfo) -> &'a str {
        self.xkb_atom_text(_type.name)
    }

    fn type_mask_txt(&self, info: &KeyTypesInfo, _type: &KeyTypeInfo) -> Option<String> {
        Some(self.mod_mask_text(&info.mods, _type.mods))
    }

    fn report_type_should_be_array(&self, _type: &KeyTypeInfo, field: &str) -> ReportedError {
        self.report_should_be_array("key type", field, self.type_txt(_type))
    }

    fn report_type_bad_type(
        &self,
        code: XkbError,
        _type: &KeyTypeInfo,
        field: &str,
        wanted: &str,
    ) -> ReportedError {
        self.report_bad_type(code.into(), "key type", field, self.type_txt(_type), wanted)
    }
}

impl KeyTypesInfo {
    fn new(include_depth: u32, mods: ModSet) -> Self {
        Self {
            mods,
            include_depth,
            errors: vec![],
            unrecoverable_error: None,
            types: vec![],
            name: None,
        }
    }

    fn find_matching_key_type(&mut self, name: Atom) -> Option<&mut KeyTypeInfo> {
        self.types.iter_mut().find(|old| old.name == name)
    }

    fn add_key_type(
        &mut self,
        ctx: &Context,
        new: KeyTypeInfo,
        same_file: bool,
    ) -> Result<(), CompileTypesError> {
        let verbosity = ctx.get_log_verbosity();

        let old = self.find_matching_key_type(new.name);

        if let Some(old) = old {
            if [MergeMode::Replace, MergeMode::Override].contains(&new.merge) {
                if (same_file && verbosity > 0) || verbosity > 9 {
                    log::warn!("{:?}: Multiple definitions of the {:?} key type; Earlier definition ignored",
                            XkbWarning::ConflictingKeyTypeDefinitions,
                            ctx.xkb_atom_text(new.name));
                }
                let _ = std::mem::replace(old, new);

                return Ok(());
            }

            if same_file {
                log::warn!(
                    "{:?}: Multiple definitions of the {:?} key type; Later definition ignored",
                    XkbWarning::ConflictingKeyTypeDefinitions,
                    ctx.xkb_atom_text(new.name)
                );
            }

            return Ok(());
        }
        self.types.push(new);
        Ok(())
    }

    fn merge_included_key_types(
        &mut self,
        ctx: &Context,
        mut from: KeyTypesInfo,
        merge: MergeMode,
    ) {
        if !from.errors.is_empty() {
            self.errors.append(&mut from.errors);
            return;
        } else if let Some(err) = from.unrecoverable_error {
            self.unrecoverable_error = Some(err);
            return;
        }

        self.mods = from.mods;

        if self.name.is_none() {
            self.name = from.name;
        }

        if self.types.is_empty() {
            self.types = from.types;
        } else {
            for mut _type in from.types {
                _type.merge = match merge {
                    MergeMode::Default => _type.merge,
                    _ => merge,
                };
                if let Err(e) = self.add_key_type(ctx, _type, false) {
                    self.errors.push(e);
                }
            }
        }
    }

    fn handle_include_keytypes(
        &mut self,
        ctx: &mut Context,
        include: IncludeStmt,
    ) -> Result<(), CompileTypesError> {
        if ctx.exceeds_include_max_depth(self.include_depth) {
            let err = CompileTypesError::ExceedsIncludeMaxDepth;
            self.unrecoverable_error = Some(err.clone());
            return Err(err);
        }

        let mut included = KeyTypesInfo::new(0, self.mods.clone());

        for stmt in include.maps {
            let file = ctx
                .process_include_file(&stmt, XkbFileType::Types)
                .map_err(|e| {
                    let e: CompileTypesError = e.into();
                    self.unrecoverable_error = Some(e.clone());
                    e
                })?;

            let mut next_incl = KeyTypesInfo::new(self.include_depth + 1, included.mods.clone());

            next_incl.handle_key_types_file(ctx, file, stmt.merge)?;

            included.merge_included_key_types(ctx, next_incl, stmt.merge);
        }

        self.merge_included_key_types(ctx, included, include.merge);

        // error handling
        if let Some(err) = self.unrecoverable_error.as_ref() {
            Err(err.clone())
        } else if !self.errors.is_empty() {
            Err(CompileTypesError::MultipleErrors(self.errors.clone()))
        } else {
            Ok(())
        }
    }

    fn set_modifiers(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        array_ndx: Option<ExprDef>,
        value: ExprDef,
    ) -> Result<(), CompileTypesError> {
        if array_ndx.is_some() {
            log::info!(
                    "The modifiers field of a key type is not an array; Illegal array subscript ignored.");
        }

        let mods = value.resolve_mod_mask(ctx, ModType::BOTH, &self.mods)
            .ok_or_else(|| {
                let err = XkbError::UnsupportedModifierMask;
                log::error!(
                    "{:?}: Key type mask field must be a modifier mask; Key type definition ignored",
                    err
                );
                CompileTypesError::KeyTypeMaskIsNotModMask

            })?;

        if _type.defined.intersects(TypeField::Mask) {
            log::warn!(
                "Multiple modifier mask definitions for key type {:?}; using {:?}, ignoring {:?}",
                ctx.xkb_atom_text(_type.name),
                ctx.type_mask_txt(self, _type),
                ctx.mod_mask_text(&self.mods, mods)
            );
        }

        _type.mods = mods;
        Ok(())
    }

    fn find_matching_map_entry(&self, _type: &KeyTypeInfo, mods: &ModMask) -> Option<usize> {
        _type
            .entries
            .iter()
            .position(|entry| entry.mods.mods == *mods)
    }

    fn add_map_entry(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        new: KeyTypeEntry,
        clobber: bool,
        report: bool,
    ) -> Result<(), CompileTypesError> {
        let type_txt = ctx.type_txt(_type);
        let old = self.find_matching_map_entry(_type, &new.mods.mods);
        if let Some(old_idx) = old {
            let old = _type.entries.get_mut(old_idx).unwrap();
            if report && (old.level != new.level) {
                log::warn!(
                    "{:?}: Multiple map entries for {:?} in {:?}; Using {:?}, ignoring {:?}",
                    XkbWarning::ConflictingKeyTypeMapEntry,
                    ctx.map_entry_txt(self, &new),
                    type_txt,
                    match clobber {
                        true => new.level,
                        false => old.level,
                    } + 1,
                    match clobber {
                        true => old.level,
                        false => new.level,
                    } + 1
                );
            } else {
                log::warn!(
                    "{:?}: Multiple occurrences of map[{:?}] = {:?} in {:?}; Ignored",
                    XkbWarning::ConflictingKeyTypeMapEntry,
                    ctx.map_entry_txt(self, &new),
                    new.level + 1,
                    ctx.type_txt(_type)
                );
                return Ok(());
            }

            if clobber {
                if new.level >= _type.num_levels {
                    _type.num_levels = new.level + 1;
                }
                old.level = new.level;
            }

            return Ok(());
        }

        if new.level >= _type.num_levels {
            _type.num_levels = new.level + 1;
        }
        _type.entries.push(new);

        Ok(())
    }

    fn set_map_entry(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        array_ndx: Option<ExprDef>,
        value: ExprDef,
    ) -> Result<(), CompileTypesError> {
        // new empty KeyTypeEntry
        let mut entry = KeyTypeEntry {
            level: 0,
            mods: Mods { mods: 0, mask: 0 },
            preserve: Mods { mods: 0, mask: 0 },
        };

        entry.mods.mods = array_ndx
            .ok_or_else(|| ctx.report_type_should_be_array(_type, "map entry"))?
            .resolve_mod_mask(ctx, ModType::BOTH, &self.mods)
            .ok_or_else(|| {
                ctx.report_type_bad_type(
                    XkbError::UnsupportedModifierMask,
                    _type,
                    "map entry",
                    "modifier mask",
                )
            })?;

        if (entry.mods.mods & !_type.mods) > 0 {
            log::warn!(
                "{:?}: Map entry for modifiers not used by type {:?}; Using {:?} instead of {:?}",
                XkbWarning::UndeclaredModifiersInKeyType,
                ctx.type_txt(_type),
                ctx.mod_mask_text(&self.mods, entry.mods.mods & _type.mods),
                ctx.map_entry_txt(self, &entry)
            );
            entry.mods.mods &= _type.mods;
        }

        entry.level = value.resolve_level(ctx)
            .ok_or_else(|| {

                let err = XkbError::UnsupportedShiftLevel;
                log::error!(
                    "{:?}: Level specifications in a key type must be integer; Ignoring malformed level specification",
                    &err);

                CompileTypesError::UnsupportedShiftLevel
            })?;

        self.add_map_entry(ctx, _type, entry, true, true)
    }

    fn add_preserve(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        mods: ModMask,
        preserve_mods: ModMask,
    ) -> Result<(), CompileTypesError> {
        let type_txt = ctx.type_txt(_type);
        for entry in _type.entries.iter_mut() {
            if entry.mods.mods != mods {
                continue;
            }
            // map exists without previous preserve
            if entry.preserve.mods == 0 {
                entry.preserve.mods = preserve_mods;
                return Ok(());
            }
            // map exists with same preserve;
            // do nothing.
            if entry.preserve.mods == preserve_mods {
                log::warn!(
                    "{:?}: Identical definitions for preserve[{:?}] in {:?}; Ignored",
                    XkbWarning::DuplicateEntry,
                    ctx.mod_mask_text(&self.mods, mods),
                    ctx.type_txt(_type)
                );
                return Ok(());
            }

            // Map exists with different preserve; latter wins.
            log::warn!(
                "{:?}: Multiple definitions for preserve[{:?}] in {:?}; Using {:?}, ignoring {:?}",
                XkbWarning::ConflictingKeyTypePreserveEntries,
                ctx.mod_mask_text(&self.mods, mods),
                type_txt,
                ctx.mod_mask_text(&self.mods, preserve_mods),
                ctx.mod_mask_text(&self.mods, entry.preserve.mods)
            );
            entry.preserve.mods = preserve_mods;
            return Ok(());
        }

        // Map does not exist, i.e. preserve[] came before map[].
        // Create a map with the specified mask mapping to Level1. The level may be overriden
        // later with an explicit map[] statement.

        // TODO: mark that these are uninitialized?
        let new = KeyTypeEntry {
            level: 0,
            mods: Mods { mods, mask: 0 },
            preserve: Mods {
                mods: preserve_mods,
                mask: 0,
            },
        };

        _type.entries.push(new);

        Ok(())
    }

    fn set_preserve(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        array_ndx: Option<ExprDef>,
        value: ExprDef,
    ) -> Result<(), CompileTypesError> {
        let mut mods = array_ndx
            .ok_or_else(|| ctx.report_type_should_be_array(_type, "preserve entry"))?
            .resolve_mod_mask(ctx, ModType::BOTH, &self.mods)
            .ok_or_else(|| {
                ctx.report_type_bad_type(
                    XkbError::UnsupportedModifierMask,
                    _type,
                    "preserve entry",
                    "modifier mask",
                )
            })?;

        if (mods & !_type.mods) != 0 {
            let before = ctx.mod_mask_text(&self.mods, mods);
            mods &= _type.mods;
            let after = ctx.mod_mask_text(&self.mods, mods);

            log::warn!(
                "{:?}: Preserve entry for modifiers not used by the {:?} type; Index {:?} converted to {:?}.",
                XkbWarning::UndeclaredModifiersInKeyType,
                ctx.type_txt(_type), before, after);
        }

        let mut preserve_mods
            = value.resolve_mod_mask(ctx, ModType::BOTH, &self.mods)
            .ok_or_else(|| {
                log::error!(
                "{:?}: Preserve value in a key type is not a modifier mask; Ignoring preserve {:?} in type {:?}.",
                XkbError::UnsupportedModifierMask,
                ctx.mod_mask_text(&self.mods, mods),
                ctx.type_txt(_type)
            );
                CompileTypesError::UnsupportedModifierMask

        })?;

        if (preserve_mods & !mods) != 0 {
            let before = ctx.mod_mask_text(&self.mods, preserve_mods);
            preserve_mods &= mods;

            let after = ctx.mod_mask_text(&self.mods, preserve_mods);

            log::warn!(
                "{:?}: Illegal value for preserve[{}] in type{:?}; Converted {} to {}.",
                XkbWarning::IllegalKeyTypePreserveResult,
                ctx.mod_mask_text(&self.mods, mods),
                ctx.type_txt(_type),
                before,
                after
            );
        }

        self.add_preserve(ctx, _type, mods, preserve_mods)
    }

    fn add_level_name(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        level: LevelIndex,
        name: Atom,
        clobber: bool,
    ) -> Result<(), CompileTypesError> {
        // See end of function for new name

        // same level, same name
        if let Some(stored_name) = _type.level_names.get(&level) {
            if *stored_name == name {
                log::warn!(
                    "{:?}: Duplicate names for level {} of key type {:?}; Ignored",
                    XkbWarning::DuplicateEntry,
                    level + 1,
                    ctx.type_txt(_type)
                );

                return Ok(());
            }
        }

        // same level, different name
        if let Some(stored_name) = _type.level_names.get(&level) {
            let old = ctx.xkb_atom_text(*stored_name);
            let new = ctx.xkb_atom_text(name);

            log::warn!(
                "{:?}: Multiple names for level {} of key type {:?}; Using {:?}, ignoring {:?}",
                XkbWarning::ConflictingKeyTypeLevelNames,
                level + 1,
                ctx.type_txt(_type),
                match clobber {
                    true => &new,
                    false => &old,
                },
                match clobber {
                    true => &old,
                    false => &new,
                }
            );

            if !clobber {
                return Ok(());
            }
        }

        // XXX: What about different level, same name?
        _type.level_names.insert(level, name);

        Ok(())
    }

    fn set_level_name(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        array_ndx: Option<ExprDef>,
        value: ExprDef,
    ) -> Result<(), CompileTypesError> {
        let level = array_ndx
            .ok_or_else(|| ctx.report_type_should_be_array(_type, "level name"))?
            .resolve_level(ctx)
            .ok_or_else(|| {
                ctx.report_type_bad_type(
                    XkbError::UnsupportedModifierMask,
                    _type,
                    "level name",
                    "integer",
                )
            })?;

        let level_name = value.resolve_string(ctx)
            .ok_or_else(|| {
                log::error!(
                        "{:?}: Non-string name for level {:?} in key type {:?}; ignoring illegal level name definition\n", XkbError::WrongFieldType, level + 1, ctx.xkb_atom_text(_type.name));
                CompileTypesError::WrongFieldType


            })?;

        self.add_level_name(ctx, _type, level, level_name, true)
    }

    fn set_keytype_field(
        &mut self,
        ctx: &Context,
        _type: &mut KeyTypeInfo,
        field: &str,
        array_ndx: Option<ExprDef>,
        value: ExprDef,
    ) -> Result<(), CompileTypesError> {
        let type_field;

        let ok = match field.to_lowercase().as_str() {
            "modifiers" => {
                type_field = TypeField::Mask;
                self.set_modifiers(ctx, _type, array_ndx, value)
            }
            "map" => {
                type_field = TypeField::Map;

                self.set_map_entry(ctx, _type, array_ndx, value)
            }
            "preserve" => {
                type_field = TypeField::Preserve;
                self.set_preserve(ctx, _type, array_ndx, value)
            }
            f if ["levelname", "level_name"].contains(&f) => {
                type_field = TypeField::LevelName;
                self.set_level_name(ctx, _type, array_ndx, value)
            }
            _ => {
                let err = XkbError::UnknownField;
                log::error!(
                    "{:?}: Unknown field {:?} in key type {:?}; definition ignored",
                    err,
                    field,
                    ctx.type_txt(_type)
                );

                type_field = TypeField::empty();
                Err(CompileTypesError::UnknownField(field.to_owned()))
            }
        };

        _type.defined |= type_field;

        ok
    }

    fn handle_keytype_body(
        &mut self,
        ctx: &Context,
        def: Vec<VarDef>,
        _type: &mut KeyTypeInfo,
    ) -> Result<(), CompileTypesError> {
        let mut ok = Ok(());

        for def in def {
            let lhs = def.name.and_then(|expr| expr.resolve_lhs(ctx));

            ok = match lhs.is_some() {
                true => Ok(()),
                false => Err(CompileTypesError::CouldNotResolveLhs),
            };

            if ok.is_err() {
                continue;
            } else if let Some(ret) = lhs {
                if let Some(elem) = ret.elem {
                    if elem.to_lowercase().as_str() == "type" {
                        log::error!("{:?}: Support for changing the default type has been removed; Statement ignored.",
                                    XkbError::InvalidSetDefaultStatement);
                    } else {
                        log::error!("{:?}: Cannot set global defaults for \"{}\" element within a key type statement: move statements to the global file scope. Assignment to \"{}.{}\" ignored.",
                            XkbError::GlobalDefaultsWrongScope,
                            elem, elem, ret.field);

                        ok = Err(CompileTypesError::InvalidFieldQualifier);
                    }

                    continue;
                }
                ok = self.set_keytype_field(ctx, _type, &ret.field, ret.index, def.value);
            }
        }

        ok
    }

    fn handle_keytype_def(
        &mut self,
        ctx: &Context,
        def: KeyTypeDef,
        merge: MergeMode,
    ) -> Result<(), CompileTypesError> {
        let mut _type = KeyTypeInfo {
            defined: TypeField::empty(),
            merge: match merge {
                MergeMode::Default => merge,
                _ => def.merge,
            },
            name: def.name,
            mods: 0,
            num_levels: 1,
            entries: vec![],
            level_names: BTreeMap::new(),
        };

        self.handle_keytype_body(ctx, def.body, &mut _type)
            .map_err(|e| {
                self.errors.push(e.clone());
                e
            })?;

        self.add_key_type(ctx, _type, true).map_err(|e| {
            self.errors.push(e.clone());
            e
        })?;

        Ok(())
    }

    fn handle_key_types_file(
        &mut self,
        ctx: &mut Context,
        file: XkbFile,
        merge: MergeMode,
    ) -> Result<(), CompileTypesError> {
        self.name = Some(file.name);

        for def in file.defs {
            let result = match def {
                Decl::Include(stmt) => self.handle_include_keytypes(ctx, stmt),
                Decl::KeyType(stmt) => self.handle_keytype_def(ctx, stmt, merge),
                Decl::Var(_) => {
                    log::error!("{:?}: Support for changing the default type has been removed; Statement ignored",
                            XkbError::WrongStatementType);

                    Ok(())
                }
                Decl::VMod(stmt) => self
                    .mods
                    .handle_vmod_def(ctx, stmt, merge)
                    .map_err(|e| e.into()),
                stmt => {
                    log::error!(
                        "{:?}: Key type files may not include other declarations; Ignoring {}",
                        XkbError::WrongStatementType,
                        stmt.stmt_type()
                    );

                    Err(CompileTypesError::WrongStatementType(stmt.stmt_type()))
                }
            };

            if let Err(e) = result {
                self.errors.push(e);
            }

            if let Some(err) = self.unrecoverable_error.as_ref() {
                return Err(err.clone());
            } else if self.errors.len() > 10 {
                let err = XkbError::InvalidSyntax;

                log::error!("{:?}: Abandoning keytypes file {:?}", err, &self.name);
                return Err(CompileTypesError::MultipleErrors(self.errors.clone()));
            }
        }

        Ok(())
    }
}

impl KeymapBuilder<TextV1> {
    fn copy_keytypes(&mut self, info: KeyTypesInfo) {
        // If no types were specified, a default unnamed one-level type
        // is used for all keys.
        let types = match info.types {
            empty if empty.is_empty() => vec![KeyType {
                mods: Mods { mods: 0, mask: 0 },
                entries: vec![],
                name: self.context.atom_intern("default"),
                num_levels: 1,
                level_names: BTreeMap::new(),
            }],
            types => types
                .into_iter()
                .map(|def| KeyType {
                    name: def.name,
                    mods: Mods {
                        mods: def.mods,
                        mask: 0,
                    },
                    num_levels: def.num_levels,

                    level_names: def.level_names,
                    entries: def.entries,
                })
                .collect(),
        };
        self.types_section_name = info.name;
        // TODO: escape
        self.types = types;
        self.mods = info.mods;
    }
}

pub(super) fn compile_keytypes(
    builder: &mut KeymapBuilder<TextV1>,
    file: XkbFile,
    merge: MergeMode,
) -> Result<(), CompileTypesError> {
    let mut info = KeyTypesInfo::new(0, builder.mods.clone());

    info.handle_key_types_file(&mut builder.context, file, merge)?;

    if let Some(err) = info.unrecoverable_error {
        return Err(err);
    } else if !info.errors.is_empty() {
        return Err(CompileTypesError::MultipleErrors(info.errors));
    }

    builder.copy_keytypes(info);

    Ok(())
}
