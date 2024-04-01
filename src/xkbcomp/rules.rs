
use super::ast::XkbFileType;

use super::xkbcomp::ComponentNames;

use crate::rust_xkbcommon::*;
use crate::errors::*;
use crate::keymap::XKB_MAX_GROUPS;
use crate::context::Context;


use std::path::PathBuf;
use std::collections::BTreeMap;



use strum::{EnumCount, IntoEnumIterator};


use logos::{Lexer,Logos};

const MAX_INCLUDE_DEPTH: usize = 5;

#[derive(Logos, Debug, PartialEq)]
#[logos(error = &'static str)]
enum RulesToken {

    #[token("!", priority=3)]
    Bang,
    
    #[token("=", priority=3)]
    Equals,

    #[token("*", priority=3)]
    Star,
    
    #[regex("[\n\r]", |_| RulesToken::EndOfLine, priority=3)]
    EndOfLine,

    #[regex(r"[ \t]+", |_| logos::Skip)]
    Whitespace,

    #[regex(r"\\[\n\r]", |_| logos::Skip, priority=5)]
    LineContinue,

    #[regex("//[^\n]*[\n\r]?", |_| logos::Skip, priority=4)]
    Comment,

    // TODO: check is_graph + other requirement
    #[regex(r"\$[A-Za-z0-9_\,\.\:\+\-\(\)!@#\$%&\?\^\*`\~\[\]\/{\}\|]+", |lex| lex.slice().parse().ok().map(|s: String| s[1..].to_owned()), priority=3)]
    GroupName(String),

    #[token("include")]
    Include,


    // TODO: check is_graph + other requirement
    #[regex(r"[A-Za-z0-9_\,\.\+/\:\-\(\)!@#\$%&\?\^\*`\~\[\]\{\}\|]+", |lex| lex.slice().parse().ok(), priority=2)]
    Identifier(String),

}

#[derive(Logos, Debug, PartialEq)]
#[logos(error = &'static str)]
enum IncludeToken {

    #[token("%%", priority=3)]
    DoublePercent,

    #[token("%H", priority=3)]
    Home,

    #[token("%S", priority=3)]
    S,
    
    #[token("%E", priority=3)]
    E,
    #[regex(r"%[.]", |lex| lex.slice().parse().ok(), priority=2)]
    UnknownFormat(String),

    #[regex("[^%]+", |lex| lex.slice().parse().ok(), priority=1)]
    OtherText(String)

}

#[derive(Clone, Copy)]
#[derive(Debug, PartialEq)]
#[derive(strum_macros::EnumCount, strum_macros::EnumIter)]
enum RulesMlvo {
    Model = 0,
    Layout = 1,
    Variant = 2,
    Option = 3
}

impl RulesMlvo {

    fn sval(&self) -> &'static str {

        use RulesMlvo::*;
        match self {
            Model => "model",
            Layout => "layout",
            Variant => "variant",
            Option => "option",

        }

    }

}


#[derive(Debug)]
#[derive(Clone, Copy)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(strum_macros::EnumCount, strum_macros::EnumIter)]
enum RulesKccgst {
    Keycodes = 0,
    Types = 1,
    Compat = 2,
    Symbols = 3,
    Geometry = 4
}

impl RulesKccgst {
    fn sval(&self) -> &'static str {
        use RulesKccgst::*;
        match self {
            Keycodes => "keycodes",
            Types => "types",
            Compat => "compat",
            Symbols => "symbols",
            Geometry => "geometry"
        }

    }
}


#[derive(Clone, Debug)]
struct MatchedSval {
    sval: String, // TODO: in original, was pointer to a slice in the input
    matched: bool
}

// A broken-down version of xkb_rule_names (without the rules, obviously)
struct MatcherRuleNames {

    model: MatchedSval,
    layouts: Vec<MatchedSval>,
    variants: Vec<MatchedSval>,
    options: Vec<MatchedSval>,

}

struct Group {
    name: String,
    elements: Vec<String>,

}

struct Mapping {
    mlvo_at_pos: [Option<RulesMlvo>; RulesMlvo::COUNT],
    num_mlvo: usize,
    defined_mlvo_mask: u32,
    layout_index: Option<LayoutIndex>,
    variant_index: Option<LayoutIndex>,
    defined_kccgst_mask: u32,
    kccgst_at_pos: [Option<RulesKccgst>; RulesKccgst::COUNT],
    num_kccgst: usize, // TODO: remove
    skip: bool
}
impl Default for Mapping {

    fn default() -> Self {
        
        Mapping {
            mlvo_at_pos:[None; RulesMlvo::COUNT],
            kccgst_at_pos:[None; RulesKccgst::COUNT],
            layout_index: None,
            variant_index: None,
            num_mlvo: 0,
            num_kccgst: 0,
            defined_kccgst_mask: 0,
            defined_mlvo_mask: 0,
            skip: false

        }
    }
}

#[derive(Clone, Copy, Debug)]
enum MlvoMatchType {
    Normal = 0,
    Wildcard,
    Group
}

struct Rule {

    mlvo_value_at_pos: Vec<String>,//RulesMlvo::COUNT],
    match_type_at_pos: Vec<MlvoMatchType>, //RulesMlvo::COUNT],
    kccgst_value_at_pos: Vec<String>, //RulesKccgst::COUNT],
    skip: bool,

}

impl Default for Rule {

    fn default() -> Self {

        Self {
            mlvo_value_at_pos: Vec::with_capacity(RulesMlvo::COUNT),
            match_type_at_pos: Vec::with_capacity(RulesMlvo::COUNT),
            kccgst_value_at_pos: Vec::with_capacity(RulesKccgst::COUNT),

            skip: false
        }
    }
}
        

struct Matcher<'c> {

    ctx: &'c Context,

    rmlvo: MatcherRuleNames,
    groups: Vec<Group>,
    kccgst: BTreeMap<RulesKccgst, String>,

    // current mapping
    mapping: Mapping,
    
    // current rule
    rule: Rule


}

impl<'c> Matcher<'c> {

    fn new(ctx: &'c Context, rmlvo: &RuleNames) -> Self {

        // TODO: don't clone the string content,
        // if possible


        // TODO: is this correct?
        let model = MatchedSval {
            sval: rmlvo.model.clone().unwrap_or_else(|| "".into()),
            matched: false,
        };
        let rmlvo = MatcherRuleNames {
            layouts: split_comma_separated_mlvo(rmlvo.layout.as_ref()),
            variants: split_comma_separated_mlvo(rmlvo.variant.as_ref()),
            options: split_comma_separated_mlvo(rmlvo.options.as_ref()), 
            model,


        };
        Self {
            ctx,
            rmlvo,
            groups: vec![],
            kccgst: BTreeMap::new(),

            mapping: Mapping::default(),
            rule: Rule::default(),

        }

    }
    
    fn group_start_new(&mut self, name: String) {

        let group = Group {
            name,
            elements: vec![] };

        self.groups.push(group);

    }

    fn group_add_element(&mut self, element: String)
    -> Result<(), &'static str> {

        let group = self.groups.iter_mut().last();

        match group {
            Some(group) => group.elements.push(element),
            None => return Err("no group to add elements to")
        }

        Ok(())
    }

    fn include(
        &mut self, 
        include_depth: usize, 
        inc: String)
    -> Result<(), String> {

        // parse the include value
        // This needs a separate lexer

        if include_depth >= MAX_INCLUDE_DEPTH {

            return Err(format!("maximum include depth ({}) exceeded; maybe there is an include loop?", MAX_INCLUDE_DEPTH));
        }

        // TODO: max size for scanner buf,
        // and checks in the loop
        let mut buf = String::new();

        for token in IncludeToken::lexer(&inc) {

            use IncludeToken::*;
            match token? {
                DoublePercent => buf += "%",
                Home => {

                    let home = match self.ctx.getenv("HOME") {
                        Some(env) => env,
                        None => return Err(format!("%H was used in an include statement, but the HOME environment variable is not set"))};

                    buf += home.as_str();
                },
                S => {
                    let default_root = self.ctx.include_path_get_system_path();

                    // TODO: limit size of scanner buf
                    buf += default_root.as_str();
                    buf += "/rules";


                },
                E => {

                    let default_root: String = self.ctx.include_path_get_extra_path();

                    buf += default_root.as_str();
                    buf += "/rules";


                },
                UnknownFormat(f) => 
                    return Err(format!("unknown % format ({}) in include statement", f)),
                OtherText(s) => buf += s.as_str()
            }
        }

        let file = std::fs::File::open(buf.clone());

        if let Ok(file) = file {
            if let Err(e) = self.read_rules_file(
                include_depth + 1, file, buf.into()) {
                todo!()
            } else {
                return Ok(());
            }
        } else {

            let error_msg = format!("{:?}: Failed to open included XKB rules \"{}\"", XkbMessageCode::NoId, buf);
            log::error!("{}", error_msg);

            return Err(error_msg);

        }

    }

    fn mapping_start_new(&mut self) {


        self.mapping = Mapping::default();

    }

    fn mapping_set_mlvo(&mut self, ident: String) {

        let pos = RulesMlvo::iter()
            .map(|mlvo| mlvo.sval())
            .position(|sval| {
                ident.len() >= sval.len() //TODO: equals?
                    && ident[..sval.len()] == *sval
            });

        // Not found
        if pos.is_none() {
            log::error!("invalid mapping: {} is not a valid value here; ignoring rule set",
                ident);
            self.mapping.skip = true;
            return;
        }

        let mlvo_pos = pos.unwrap();

        if (self.mapping.defined_mlvo_mask & (1 << mlvo_pos)) != 0 {
            log::error!("invalid mapping: {}.{} appears twice on the same line; ignoring rule set",
                todo!(), todo!());
            self.mapping.skip = true;
            return;

        }

        // If there are leftovers still, it must be an index.
        let mlvo = RulesMlvo::iter()
            .nth(mlvo_pos).unwrap();
        let mlvo_sval = mlvo.sval();

        if mlvo_sval.len() < ident.len() {
            let (idx, consumed) = match extract_layout_index(
                &ident[mlvo_sval.len()..], 
                ) {
                Some((idx, consumed)) => (Some(idx), Some(consumed)),
                None => (None, None) };

           
            if let Some(consumed) = consumed {
                if (ident.len() - mlvo_sval.len()) != consumed {
                    log::error!("invalid mapping: \"{}.{}\" may only be followed by a valid group index; ignoring rule set",
                        mlvo_sval.len(),
                        todo!());
                    self.mapping.skip = true;
                    return;
                }
            }

            if mlvo == RulesMlvo::Layout {
                self.mapping.layout_index = idx;
            }
            else if mlvo == RulesMlvo::Variant {
                self.mapping.variant_index = idx;
            }
            else {
                log::error!("invalid mapping: \"{}.{}\" cannot be followed by a group index; ignoring rule set",
                    mlvo_sval.len(),
                    todo!());
                self.mapping.skip = true;
                return;

            }
        }
       
        // TODO: check array bounds
        self.mapping.mlvo_at_pos[
            self.mapping.num_mlvo] = Some(mlvo);

        self.mapping.defined_mlvo_mask |= 1 << mlvo_pos;
        self.mapping.num_mlvo += 1;

        
    }

    fn mapping_set_kccgst(&mut self, ident: String) {

        let pos = RulesKccgst::iter()
            .map(|kccgst| kccgst.sval())
            .position(|sval| {
                ident.len() >= sval.len()
                && ident[..sval.len()] == *sval
            });

        // Not found
        if pos.is_none() {
            log::error!("invalid mapping: {}.{} is not a valid value here; ignoring rule set",
                todo!(), todo!());
            self.mapping.skip = true;
            return;

        }
        let kccgst_pos = pos.unwrap();

        if self.mapping.defined_kccgst_mask & (1 << kccgst_pos) != 0 {
            log::error!("invalid mapping {}.{} appears twice on the same line; ignoring rule set",
                todo!(), todo!());
            self.mapping.skip = true;
            return;

        }

        // TODO: check array bounds
        self.mapping.kccgst_at_pos[
            self.mapping.num_kccgst] = RulesKccgst::iter().nth(kccgst_pos);
        self.mapping.num_kccgst += 1;
        self.mapping.defined_kccgst_mask |= 1 << kccgst_pos;
        


    }

    fn mapping_verify(&mut self) {

        if self.mapping.num_mlvo == 0 {
            log::debug!("invalid mapping: must have at least one value on the left hand side; ignoring rule set");
            self.mapping.skip = true;
            return;
        }
        if self.mapping.num_kccgst == 0 {
            log::debug!("invalid mapping: must have at least one value on the right hand side; ignoring rule set");
            self.mapping.skip = true;
            return;
        }

        // "The following is very stupid, but this is how it works. See the 'Notes' section in the
        // overview above"

        if (self.mapping.defined_mlvo_mask & (1 << RulesMlvo::Layout as u32)) != 0 {
            if self.mapping.layout_index.is_none() {

                if self.rmlvo.layouts.len() > 1 {
                    self.mapping.skip = true;
                    return;
                }
            } else if let Some(layout_idx)
                = self.mapping.layout_index {
                if self.rmlvo.layouts.len() == 1
                    || layout_idx >= self.rmlvo.layouts.len() {
                        self.mapping.skip = true;
                        return;
                }
            }

        }
        if (self.mapping.defined_mlvo_mask & (1 << RulesMlvo::Variant as u32)) != 0 {
            if self.mapping.variant_index.is_none() {

                if self.rmlvo.variants.len() > 1 {
                    self.mapping.skip = true;
                    return;
                }
            } else if let Some(variant_idx)
                = self.mapping.variant_index {
                if self.rmlvo.variants.len() == 1
                    || variant_idx >= self.rmlvo.variants.len() {
                        self.mapping.skip = true;
                        return;
                }

            }
        }
    
        return

    }

    fn rule_start_new(&mut self) {

        self.rule = Rule::default();
        self.rule.skip = self.mapping.skip;

    }

    fn rule_set_mlvo_common(
        &mut self,
        ident: String,
        match_type: MlvoMatchType) {

        if self.rule.mlvo_value_at_pos.len() + 1
           > self.mapping.num_mlvo {

               log::error!("invalid rule: has more values than the mapping line; ignoring rule");
               self.rule.skip = true;
               return;

        }
        // TODO: check bounds
        self.rule.match_type_at_pos.push(match_type);
        self.rule.mlvo_value_at_pos.push(ident);


    }
    fn rule_set_mlvo_wildcard(
        &mut self) {

        let dummy = String::new();
        self.rule_set_mlvo_common(dummy, MlvoMatchType::Wildcard)

    }

    fn rule_set_mlvo_group(
        &mut self, ident: String) {
 
        self.rule_set_mlvo_common(ident, MlvoMatchType::Group)

    }

    fn rule_set_mlvo(&mut self, ident: String) {

        self.rule_set_mlvo_common(ident, MlvoMatchType::Normal)

    }

    fn rule_set_kccgst(&mut self, ident: String) {

        if self.rule.kccgst_value_at_pos.len() + 1 
            > self.mapping.num_kccgst {
                log::error!("invalid rule: has more values than the mapping line; ignoring rule");
                self.rule.skip = true;
                return;
        }

        // TODO: check bounds
        self.rule.kccgst_value_at_pos.push(ident);
    }


    fn match_group(&self, group_name: &str, elem_to_find: &str)
    -> bool {

        let mut found_group = None;

        for group in self.groups.iter() {
            if group.name == group_name {
                found_group = Some(group);
                break;
            }
        }

        if let Some(found_group) = found_group {
           
            for element in found_group.elements.iter() {
                if elem_to_find == element {
                    return true;
                }
            }

            false

        }
        else  {
            // rules/evded intentionally uses some undeclared
            // group names in rules (e.g. commented group definitions
            // which may be uncommented if needed).
            // So we continue silently

            false
        }


    }

    fn match_value(&self, val: &str, to: &str,
        match_type: MlvoMatchType) -> bool {

        use MlvoMatchType::*;
        match match_type {
            Wildcard => true,
            Group => self.match_group(val,to),
            _ => val == to
        }

    }

    fn match_value_and_mark(
        &self, val: &str, to: &mut MatchedSval,
        match_type: MlvoMatchType) -> bool {

        let matched = self.match_value(
            val, to.sval.as_ref(), match_type);

        if matched {
            to.matched = true;
        }

        matched

    }

    /// This function performs %-expansion on `value`,
    /// and appends the value to `self.kccgst[category] 
    fn append_expanded_kccgst_value(&mut self,
        category: &RulesKccgst, value: String)
    -> Result<(), &'static str> {


        // TODO: implement this with a Logos lexer

        let mut expanded = String::new();

        let mut chars = value.chars().enumerate().peekable();
        
        let mut current_ch = chars.next()
                .map(|(i,ch)| ch);

        while let Some(mut ch) = current_ch {

            
            if ch != '%' {
                expanded.push(ch);
                current_ch = chars.next()
                    .map(|(i,ch)| ch);
                continue;
            }
            ch = match chars.next() {
                Some((_,ch)) => ch,
                None => return Err(todo!()) };


            let mut prefix = None;
            let mut suffix = None;

            // Check for prefix
            if ['(','+','|', '_','-'].contains(&ch) {

                prefix = Some(ch);
                if '(' == ch { suffix = Some(')'); }
                ch = match chars.next() {
                    Some((_,ch)) => ch,
                    None => return Err(todo!()) };

            }


            // Mandatory model/layout/variant specifier
            let mlv = match ch {
                'm' => RulesMlvo::Model,
                'l' => RulesMlvo::Layout,
                'v' => RulesMlvo::Variant,
                _ => return Err(todo!()) };

            // Check for index
            let mut index = None;
            if let Some((mut i, mut ch)) = chars.next() {

                if ch == '[' {
                    
                    if mlv != RulesMlvo::Layout
                        && mlv != RulesMlvo::Variant {
                            log::error!("invalid index in %-expansion; may only index layout or variant");
                            return Err(todo!());
                    }
                    
                    let (idx, consumed) = match
                        extract_layout_index(&value[i..]) {
                            Some(c) => c,
                            None => return Err(todo!()) };

                        index = Some(idx);

                        for _ in 0..consumed {
                            (i, ch) = chars.next().unwrap();
                            current_ch = Some(ch);
                        }
                    

                } else { 
                    current_ch = Some(ch);
                }

                // Check for suffix, if there is supposed to be one
                if let Some(sfx) = suffix {
                        if current_ch != Some(sfx) {
                            return Err(todo!()); }

                    current_ch = chars.next()
                        .map(|(i,ch)| ch);
                }
            } else { return Err(todo!()); }

            // Get the expanded value
            let mut expanded_value = None;

            if mlv == RulesMlvo::Layout {

                if let Some(idx) = index {
                    if let Some(layout) 
                        = self.rmlvo.layouts.get_mut(idx) {

                            expanded_value = Some(layout);

                    }
                } else if index.is_none()
                    && self.rmlvo.layouts.len() == 1 {
                        expanded_value = self.rmlvo.layouts.get_mut(0);
                }
            }
            else if mlv == RulesMlvo::Variant {
                if let Some(idx) = index {
                    if let Some(variant)
                        = self.rmlvo.variants.get_mut(idx) {

                            expanded_value = Some(variant);
                    }
                } else if index.is_none()
                    && self.rmlvo.variants.len() == 1 {
                        
                        expanded_value = self.rmlvo.variants.get_mut(0);
                }
            }
            else if mlv == RulesMlvo::Model {

                expanded_value = Some(&mut self.rmlvo.model);

            }

            // If we didn't get one, skip silently.
            let expanded_value = match expanded_value {
                Some(s) if s.sval.len() > 0 => s,
                _ => continue };

            if let Some(pfx) = prefix {
                
                expanded.push(pfx);
            }

            for c in expanded_value.sval.chars() {
                expanded.push(c);
            }

            if let Some(sfx) = suffix {

                expanded.push(sfx);
            }

            expanded_value.matched = true;
            

        }

        // insert if does not exist
        let to = match self.kccgst.get_mut(category) {
            Some(s) => s,
            None => {
                self.kccgst.insert(*category, String::new());
                self.kccgst.get_mut(category).unwrap() }}

        ;
        // Appends

        let ch = expanded.chars().next();
        let expanded_plus = [Some('+'), Some('|')].contains(&ch);
        let ch = to.chars().next();
        let to_plus = [Some('+'), Some('|')].contains(&ch);
       

        if expanded_plus || to.is_empty() {
            *to += expanded.as_str();
        }
        else if to_plus {
            *to = expanded + to.as_str(); 
        }

        

        Ok(())

    }

    fn rule_verify(&mut self) {

     if self.rule.mlvo_value_at_pos.len()
         != self.mapping.num_mlvo
        || self.rule.kccgst_value_at_pos.len()
            != self.mapping.num_kccgst {

                log::error!("invalid rule: must have same number of values as mapping line; ignoring rule");
                self.rule.skip = true;
}
 }

    fn rule_apply_if_matches(&mut self)
    -> Result<(), &'static str> {

        for i in 0..self.mapping.num_mlvo {

            // TODO: check index validity
            let mlvo = self.mapping.mlvo_at_pos[i];
let value = &self.rule.mlvo_value_at_pos[i];
            let match_type = self.rule.match_type_at_pos[i];

            let mut matched = false;

            if mlvo == Some(RulesMlvo::Model) {

                let mut to_str = self.rmlvo.model.clone();
                // TODO: more functional style so don't need to clone
                matched = self.match_value_and_mark(value, &mut to_str, match_type);
                self.rmlvo.model = to_str;


            } else if mlvo == Some(RulesMlvo::Layout) {

                let idx = match self.mapping.layout_index {
                    None => 0, Some(idx) => idx };
                let mut to_str = self.rmlvo.layouts.get(idx)
                    .ok_or("Invalid layout index")?.clone();
                matched = self.match_value_and_mark(value, &mut to_str, match_type);
                self.rmlvo.layouts[idx] = to_str;

        
            } else if mlvo == Some(RulesMlvo::Variant) {

                let idx = match self.mapping.variant_index {
                    None => 0, Some(idx) => idx };
                let mut to_str = self.rmlvo.variants.get(idx)
                    .ok_or("Invalid variant index")?.clone();
                matched = self.match_value_and_mark(value, &mut to_str, match_type);
                self.rmlvo.variants[idx] = to_str;

            } else if mlvo == Some(RulesMlvo::Option) {

                for i in 0..self.rmlvo.options.len() {

                    let mut to_str = self.rmlvo.options[i].clone();
                    matched = self.match_value_and_mark(value, &mut to_str, match_type);
                    self.rmlvo.options[i] = to_str;

                    if matched {
                        break;
                    }
                }
            }

            if !matched {
                return Ok(());
            }


        }
        for i in 0..self.mapping.num_kccgst {

            // TODO: reconsider these data structures
            let kccgst = self.mapping.kccgst_at_pos[i].unwrap();
            let value = self.rule.kccgst_value_at_pos[i].clone();

            self.append_expanded_kccgst_value(
                &kccgst, value)?;

        }

        // If a rule matches in a rule set, the rest of the set
        // should be skipped. However, rule sets matching against
        // options may contain several legitimate rules,
        // so they are processed entirely.

        if self.mapping.defined_mlvo_mask & (1 << RulesMlvo::Option as u32) == 0 {
            self.mapping.skip = true;
        }
    
        Ok(())

    }


}

fn extract_layout_index(s: &str) 
    -> Option<(LayoutIndex,usize)> {

    // "This function is pretty stupid, but works for now"
    if s.len() < 3 { return None; }
   
    let mut chars = s.chars();
    let c0 = chars.next().unwrap();
    let c1 = chars.next().unwrap();
    let c2 = chars.next().unwrap();
    if c0 != '['
        || !c1.is_ascii_digit()
     || c2 != ']' {
            return None;

    }

    let c1_u32 = u32::from(c1);
    let zero_u32 = u32::from('0');

    if c1_u32 - zero_u32  < 1
        || c1_u32 - zero_u32 > XKB_MAX_GROUPS.into() {
            return None;
    }

    // to zero-based index
    let layout_index = c1_u32 - zero_u32 - 1;
    let layout_index: usize = layout_index.try_into().unwrap();

    Some((layout_index, 3))
}

fn split_comma_separated_mlvo(s: Option<&String>) -> Vec<MatchedSval> {

    let s = match s {
        Some(s) => s,
        None => "" };

    let substrings: Vec<String> = s.split(",")
    .map(|s| s.to_owned()).collect();

    // TODO: Make sure the array returned by this function always includes at least one value.

    let strings = match substrings.len() {
        0 => vec!["".into()],
        _ => substrings };

    strings.into_iter()
        .map(|s| MatchedSval { sval: s, matched: false })
        .collect()
}

#[derive(PartialEq, Debug)]
enum MatcherState {
    Initial,
    Bang,
    GroupName,
    GroupElement,
    IncludeStatement,
    MappingMlvo,
    MappingKccgst,
    RuleMlvoFirst,
    RuleMlvo,
    RuleMlvoNoTok(
        //pass lexer.next() from last iteration
        Option<Result<RulesToken,&'static str>>
    ),
    RuleKccgst,
    Unexpected,
    Finish
}
impl<'c> Matcher<'c> {

    fn state_machine(
        &mut self, 
        mut lexer: Lexer<RulesToken>,
        include_depth: usize)
    -> Result<(), &'static str> {

        use MatcherState::*;
        let mut state = Initial;
        while state != Finish {
            match state {

                Initial => match lexer.next() {
                    Some(Ok(RulesToken::Bang)) => state = Bang,
                    Some(Ok(RulesToken::EndOfLine)) => state = Initial,
                    None => state = Finish,
                    _ => state = Unexpected
                },

                Bang => match lexer.next() {
                    Some(Ok(RulesToken::GroupName(s))) => {
                        self.group_start_new(s);
                        state = GroupName },
                    Some(Ok(RulesToken::Include)) => state = IncludeStatement,
                    Some(Ok(RulesToken::Identifier(s))) => {
                        self.mapping_start_new();
                        self.mapping_set_mlvo(s);
                        state = MappingMlvo; },
                    _ => state = Unexpected
                },

                GroupName => match lexer.next() {
                    Some(Ok(RulesToken::Equals)) => state = GroupElement,
                    _ => state = Unexpected
                },

                GroupElement => match lexer.next() {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        self.group_add_element(s);
                        state = GroupElement; },
                    Some(Ok(RulesToken::EndOfLine)) => state = Initial,
                    _ => state = Unexpected
                },

                IncludeStatement => match lexer.next() {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        self.include(include_depth, s);
                        state = Initial; },
                    _ => state = Unexpected
                },

                MappingMlvo => match lexer.next() {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        if !self.mapping.skip { self.mapping_set_mlvo(s); }
                        state = MappingMlvo; },
                    Some(Ok(RulesToken::Equals)) => state = MappingKccgst,
                    _ => state = Unexpected 
                },

                MappingKccgst => match lexer.next() {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        if !self.mapping.skip { self.mapping_set_kccgst(s); }
                        state = MappingKccgst; },
                    Some(Ok(RulesToken::EndOfLine)) => {
                        if !self.mapping.skip { self.mapping_verify(); }
                        state = RuleMlvoFirst; },
                    _ => state = Unexpected
                },

                RuleMlvoFirst => match lexer.next() {
                    Some(Ok(RulesToken::Bang)) => state = Bang,
                    Some(Ok(RulesToken::EndOfLine)) => state = RuleMlvoFirst,
                    None => state = Finish,
                    token => { 
                        self.rule_start_new();
                        state = RuleMlvoNoTok(token);
                    },
                },

                RuleMlvo => state = RuleMlvoNoTok(lexer.next()), 

                RuleMlvoNoTok(token) => match token {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        if !self.rule.skip { self.rule_set_mlvo(s); }
                        state = RuleMlvo; },
                    Some(Ok(RulesToken::Star)) => {
                       if !self.rule.skip { self.rule_set_mlvo_wildcard(); }
                       state = RuleMlvo; },
                    Some(Ok(RulesToken::GroupName(s))) => {
                        if !self.rule.skip { self.rule_set_mlvo_group(s); }
                        state = RuleMlvo; },
                    Some(Ok(RulesToken::Equals)) => state = RuleKccgst,
                    _ => state = Unexpected
                },

                RuleKccgst => match lexer.next() {
                    Some(Ok(RulesToken::Identifier(s))) => {
                        if !self.rule.skip { self.rule_set_kccgst(s); }
                        state = RuleKccgst; },
                    Some(Ok(RulesToken::EndOfLine)) => {
                        if !self.rule.skip { self.rule_verify(); }
                        if !self.rule.skip { self.rule_apply_if_matches()?; }
                        state = RuleMlvoFirst; },
                    _ => state = Unexpected,
                },

                Unexpected => return Err("unexpected token"),

                Finish => return Ok(())
                // TODO: different error handling for logos lexer errors?

            
            }
        }

        Ok(())


    }

}


impl<'l> Matcher<'l> {

    fn read_rules_file(
        &mut self,
        include_depth: usize,
        mut file: std::fs::File,
        path: PathBuf)
        -> Result<(), KeymapErr>
    {

        use std::io::prelude::*;
        // TODO: use map_file?
        let mut string = String::new();
        if let Err(e) = file.read_to_string(&mut string) {
            log::error!("{:?}: Couldn't read rules file {:?}: {}", XkbMessageCode::NoId,
                path, e);
            return Err(KeymapErr::Io(e));
        };

        // equivalent of scanner_init
        let lexer = RulesToken::lexer(&string);


        self.state_machine(lexer, include_depth)
            .map_err(|e| KeymapErr::MatchError(e))
        

    }

}


impl ComponentNames {

    pub(crate) fn from_rules(
        context: &mut Context,
        rule_names: &RuleNames)
        -> Result<Self, KeymapErr> {

        let opt_file = context.find_file_in_xkb_path(
            rule_names.rules.as_ref(), 
            XkbFileType::Rules,
            &mut 0);


        let mut matcher = Matcher::new(context, rule_names);
        
        if let Some((path, file)) = opt_file {

            let result = matcher.read_rules_file( 0, file, path.clone());

            if result.is_err() 
            || matcher.kccgst.get(&RulesKccgst::Keycodes).is_none() //keycodes 
            || matcher.kccgst.get(&RulesKccgst::Types).is_none() //types
            || matcher.kccgst.get(&RulesKccgst::Compat).is_none() //compat
            || matcher.kccgst.get(&RulesKccgst::Symbols).is_none() //symbols
            {


                log::error!("{:?}: No components returned from XKB rules {:?}", XkbMessageCode::NoId, path);

                if let Err(e) = result {
                    return Err(e); }
                else {
                    return Err(KeymapErr::RulesNoComponentsReturned); }

                

            }
        }
        

        let out = ComponentNames {
            keycodes: matcher.kccgst.get(&RulesKccgst::Keycodes).cloned().unwrap_or_else(|| "".into()),
            types: matcher.kccgst.get(&RulesKccgst::Types).cloned().unwrap_or_else(|| "".into()),
            compat: matcher.kccgst.get(&RulesKccgst::Compat).cloned().unwrap_or_else(|| "".into()),
            symbols: matcher.kccgst.get(&RulesKccgst::Symbols).cloned().unwrap_or_else(|| "".into()),

        };

        //TODO: logging

        return Ok(out);

    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lex_rmlvo() {


        crate::log_init!();
        let path: &'static str = "./test/data/rules/evdev";
        let string = std::fs::read_to_string(path).unwrap();
        for token in RulesToken::lexer(&string) {
            assert!(token.is_ok());
        }


    }

}
