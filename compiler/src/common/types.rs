use crate::common::token::*;
use std::fmt::Display;
use std::rc::Rc;
pub use struct_ref::StructRef;

static RETURN_REG: &[&str; 3] = &["%al", "%eax", "%rax"];

pub trait TypeInfo {
    // returns size in bytes of type
    fn size(&self) -> usize;

    // returns the correct suffix for a register of type
    fn reg_suffix(&self) -> String;

    // returns the instruction-suffixes
    fn suffix(&self) -> String;

    // returns the instruction-suffixes spelled out
    fn complete_suffix(&self) -> String;

    // returns the return register name of type
    fn return_reg(&self) -> String;
}

#[derive(Clone, PartialEq, Debug)]
pub enum NEWTypes {
    Primitive(Types),
    Array { amount: usize, of: Box<NEWTypes> },
    Pointer(Box<NEWTypes>),
    Struct(StructInfo),
    Union(StructInfo),
    Enum(Option<String>, Vec<(Token, i32)>),
}

// this code is shamelessly copied from the more sophisticated saltwater compiler
mod struct_ref {
    use super::NEWTypes;
    use super::Token;
    use super::TokenType;
    use std::cell::RefCell;
    use std::rc::Rc;

    type IsComplete = bool;
    type InDefinition = bool;

    thread_local! {
        static CUSTOMS: RefCell<Vec<Rc<Vec<(NEWTypes, Token)>>>> = Default::default();
        static CUSTOMS_INFO: RefCell<Vec<(IsComplete,InDefinition)>> = Default::default();
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct StructRef {
        index: usize,
        kind: TokenType,
    }

    impl StructRef {
        pub fn new(kind: TokenType, is_definition: bool) -> StructRef {
            CUSTOMS_INFO.with(|list| {
                list.borrow_mut().push((false, is_definition));
            });
            CUSTOMS.with(|list| {
                let mut types = list.borrow_mut();
                let index = types.len();
                types.push(Rc::new(vec![]));

                StructRef { index, kind }
            })
        }
        pub fn get_kind(&self) -> &TokenType {
            &self.kind
        }

        pub fn get(&self) -> Rc<Vec<(NEWTypes, Token)>> {
            CUSTOMS.with(|list| list.borrow()[self.index].clone())
        }
        pub(crate) fn update(&self, members: Vec<(NEWTypes, Token)>) {
            CUSTOMS_INFO.with(|list| {
                let mut types = list.borrow_mut();
                types[self.index].0 = true;
            });
            CUSTOMS_INFO.with(|list| {
                let mut types = list.borrow_mut();
                types[self.index].1 = false;
            });
            CUSTOMS.with(|list| {
                let mut types = list.borrow_mut();
                types[self.index] = members.into();
            });
        }
        pub fn is_complete(&self) -> bool {
            CUSTOMS_INFO.with(|list| list.borrow()[self.index].0)
        }
        pub fn in_definition(&self) -> bool {
            CUSTOMS_INFO.with(|list| list.borrow()[self.index].1)
        }

        pub fn being_defined(&self) {
            CUSTOMS_INFO.with(|list| list.borrow_mut()[self.index].1 = true)
        }
    }
}
#[derive(Clone, PartialEq, Debug)]
pub enum StructInfo {
    Named(String, StructRef),
    Anonymous(Vec<(NEWTypes, Token)>),
}
impl StructInfo {
    pub fn members(&self) -> Rc<Vec<(NEWTypes, Token)>> {
        match self {
            StructInfo::Named(_, s) => s.get(),
            StructInfo::Anonymous(m) => Rc::new(m.clone()),
        }
    }
    pub fn member_offset(&self, member_to_find: &str) -> usize {
        self.members()
            .iter()
            .take_while(|(_, name)| name.unwrap_string() != member_to_find)
            .fold(0, |acc, (t, _)| acc + t.size())
    }
    pub fn member_type(&self, member_to_find: &str) -> NEWTypes {
        self.members()
            .iter()
            .find(|(_, name)| name.unwrap_string() == member_to_find)
            .unwrap()
            .0
            .clone()
    }
    fn name(&self) -> &str {
        match self {
            StructInfo::Named(name, _) => name,
            StructInfo::Anonymous(_) => "<anonymous>",
        }
    }
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Named(_, s) => s.is_complete(),
            Self::Anonymous(_) => true,
        }
    }
}

impl TypeInfo for NEWTypes {
    fn size(&self) -> usize {
        match self {
            NEWTypes::Primitive(t) => t.size(),
            NEWTypes::Struct(s) => s.members().iter().fold(0, |acc, (t, _)| acc + t.size()),
            NEWTypes::Union(_) => self.union_biggest().size(),
            NEWTypes::Pointer(_) => NEWTypes::Primitive(Types::Long).size(),
            NEWTypes::Enum(..) => NEWTypes::Primitive(Types::Int).size(),
            NEWTypes::Array { amount, of: element_type } => amount * element_type.size(),
        }
    }
    fn reg_suffix(&self) -> String {
        match self {
            NEWTypes::Primitive(t) => t.reg_suffix(),
            NEWTypes::Union(_) => self.union_biggest().reg_suffix(),
            NEWTypes::Enum(..) => NEWTypes::Primitive(Types::Int).reg_suffix(),
            NEWTypes::Pointer(_) | NEWTypes::Array { .. } | NEWTypes::Struct(..) => {
                NEWTypes::Primitive(Types::Long).reg_suffix()
            }
        }
    }
    fn suffix(&self) -> String {
        match self {
            NEWTypes::Primitive(t) => t.suffix(),
            NEWTypes::Union(_) => self.union_biggest().suffix(),
            NEWTypes::Enum(..) => NEWTypes::Primitive(Types::Int).suffix(),
            NEWTypes::Pointer(_) | NEWTypes::Array { .. } | NEWTypes::Struct(..) => {
                NEWTypes::Primitive(Types::Long).suffix()
            }
        }
    }
    fn complete_suffix(&self) -> String {
        match self {
            NEWTypes::Primitive(t) => t.complete_suffix(),
            NEWTypes::Union(_) => self.union_biggest().complete_suffix(),
            NEWTypes::Enum(..) => NEWTypes::Primitive(Types::Int).complete_suffix(),
            NEWTypes::Pointer(_) | NEWTypes::Array { .. } | NEWTypes::Struct(..) => {
                NEWTypes::Primitive(Types::Long).complete_suffix()
            }
        }
    }
    fn return_reg(&self) -> String {
        match self {
            NEWTypes::Primitive(t) => t.return_reg(),
            NEWTypes::Pointer(_) | NEWTypes::Array { .. } => {
                NEWTypes::Primitive(Types::Long).return_reg()
            }
            NEWTypes::Enum(..) => NEWTypes::Primitive(Types::Int).return_reg(),
            NEWTypes::Union(..) => self.union_biggest().return_reg(),
            NEWTypes::Struct(..) => unimplemented!("currently can't return structs"),
        }
    }
}
impl Display for NEWTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NEWTypes::Primitive(t) => t.fmt().to_string(),
                NEWTypes::Array { of, amount } => format!("{}[{}]", of, amount),
                NEWTypes::Pointer(to) => format!("{}*", to),
                NEWTypes::Union(s) => "union ".to_string() + s.name(),
                NEWTypes::Struct(s) => "struct ".to_string() + s.name(),
                NEWTypes::Enum(Some(name), ..) => "enum ".to_string() + name,
                NEWTypes::Enum(None, ..) => "enum <anonymous>".to_string(),
            }
        )
    }
}
#[macro_export]
macro_rules! arr_decay {
    ($arr:expr,$ast:expr,$token:expr) => {
        if let NEWTypes::Array { of, .. } = $arr {
            $arr = NEWTypes::Pointer(of);

            $ast.kind = ExprKind::Unary {
                token: Token { token: TokenType::Amp, ..$token.clone() },
                right: Box::new($ast.clone()),
            };
        }
    };
}
impl NEWTypes {
    pub fn default() -> NEWTypes {
        NEWTypes::Primitive(Types::Int)
    }
    pub fn pointer_to(&mut self) {
        *self = NEWTypes::Pointer(Box::new(self.clone()));
    }
    pub fn deref_at(&self) -> Option<NEWTypes> {
        match self {
            NEWTypes::Pointer(inner) => Some(*inner.clone()),
            _ => None,
        }
    }
    pub fn is_void(&self) -> bool {
        *self == NEWTypes::Primitive(Types::Void)
    }
    pub fn is_ptr(&self) -> bool {
        matches!(*self, NEWTypes::Pointer(_))
    }
    pub fn type_compatible(&self, other: &NEWTypes) -> bool {
        match (self, other) {
            (NEWTypes::Primitive(Types::Void), NEWTypes::Primitive(Types::Void)) => true,

            (NEWTypes::Primitive(Types::Void), NEWTypes::Primitive(_))
            | (NEWTypes::Primitive(_), NEWTypes::Primitive(Types::Void)) => false,

            (NEWTypes::Primitive(_), NEWTypes::Primitive(_)) => true,

            (NEWTypes::Pointer(l), NEWTypes::Pointer(r))
                if matches!(**l, NEWTypes::Struct(..)) && matches!(**r, NEWTypes::Struct(..)) =>
            {
                l.type_compatible(r)
            }
            // void* is compatible to any other pointer
            (NEWTypes::Pointer(t), NEWTypes::Pointer(_))
            | (NEWTypes::Pointer(_), NEWTypes::Pointer(t))
                if matches!(**t, NEWTypes::Primitive(Types::Void)) =>
            {
                true
            }

            (NEWTypes::Pointer(_), NEWTypes::Pointer(_)) => *self == *other,

            // two structs/unions are compatible if they have the same name and members
            (NEWTypes::Struct(s_l), NEWTypes::Struct(s_r))
            | (NEWTypes::Union(s_l), NEWTypes::Union(s_r)) => {
                if let (StructInfo::Named(name_l, _), StructInfo::Named(name_r, _)) = (s_l, s_r) {
                    let matching_members = s_l
                        .members()
                        .iter()
                        .zip(s_r.members().iter())
                        .filter(|(l, r)| l.0 == r.0 && l.1.unwrap_string() == r.1.unwrap_string())
                        .count();
                    *name_l == *name_r
                        && matching_members == s_l.members().len()
                        && matching_members == s_r.members().len()
                } else {
                    false
                }
            }
            (NEWTypes::Enum(..), NEWTypes::Primitive(Types::Void))
            | (NEWTypes::Primitive(Types::Void), NEWTypes::Enum(..)) => false,

            (NEWTypes::Enum(..), NEWTypes::Primitive(_))
            | (NEWTypes::Primitive(_), NEWTypes::Enum(..)) => true,

            _ => false,
        }
    }
    pub fn is_scalar(&self) -> bool {
        match self {
            NEWTypes::Primitive(Types::Void) => false,
            NEWTypes::Primitive(_) | NEWTypes::Pointer(_) | NEWTypes::Enum(..) => true,
            _ => false,
        }
    }
    pub fn is_integer(&self) -> bool {
        match self {
            NEWTypes::Primitive(Types::Void) => false,
            NEWTypes::Primitive(_) | NEWTypes::Enum(..) => true,
            _ => false,
        }
    }
    pub fn is_aggregate(&self) -> bool {
        match self {
            NEWTypes::Struct(_) | NEWTypes::Union(_) => true,
            _ => false,
        }
    }
    fn union_biggest(&self) -> NEWTypes {
        match self {
            NEWTypes::Union(s) => s
                .members()
                .iter()
                .max_by_key(|(type_decl, _)| type_decl.size())
                .expect("union can't be empty, checked in parser")
                .0
                .clone(),
            _ => unreachable!("not union"),
        }
    }
    // used in parser to check if type contains any incomplete type when accessing it's members
    // would be better to check in typechecker but at that point type could be comeplete
    pub fn is_complete(&self) -> bool {
        match self {
            NEWTypes::Struct(s) | NEWTypes::Union(s) => s.is_complete(),
            NEWTypes::Array { of: to, .. } => to.is_complete(),
            _ if self.is_void() => false,
            _ => true,
        }
    }

    pub fn max(&self) -> i64 {
        match self {
            NEWTypes::Primitive(t) => t.max(),
            NEWTypes::Pointer(_) => i64::MAX,
            _ => unreachable!(),
        }
    }
    pub fn min(&self) -> i64 {
        match self {
            NEWTypes::Primitive(t) => t.min(),
            NEWTypes::Pointer(_) => i64::MIN,
            _ => unreachable!(),
        }
    }

    pub fn get_primitive(&self) -> Option<&Types> {
        if let NEWTypes::Primitive(type_decl) = self {
            Some(type_decl)
        } else {
            None
        }
    }
}

// converts token of aggregate type into its corresponding type
#[macro_export]
macro_rules! into_newtype {
    ($token:expr,$name:expr,$value:expr) => {
        match $token {
            TokenType::Struct => NEWTypes::Struct(StructInfo::Named($name, $value.unwrap_aggr())),
            TokenType::Union => NEWTypes::Union(StructInfo::Named($name, $value.unwrap_aggr())),
            TokenType::Enum => NEWTypes::Enum(Some($name), $value.unwrap_enum()),
            _ => unreachable!("should only be used for aggregate types"),
        }
    };
    ($token:expr,$value:expr) => {
        match $token {
            TokenType::Struct => NEWTypes::Struct(StructInfo::Anonymous($value)),
            TokenType::Union => NEWTypes::Union(StructInfo::Anonymous($value)),
            _ => unreachable!("should only be used for aggregate types"),
        }
    };
    ($value:expr) => {
        NEWTypes::Enum(None, $value)
    };
}
#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub enum Types {
    Void,
    Char,
    Int,
    Long,
}

impl TypeInfo for Types {
    // returns type-size in bytes
    fn size(&self) -> usize {
        match self {
            Types::Void => 0,
            Types::Char => 1,
            Types::Int => 4,
            Types::Long => 8,
        }
    }
    fn reg_suffix(&self) -> String {
        String::from(match self {
            Types::Void => unreachable!(),
            Types::Char => "b",
            Types::Int => "d",
            Types::Long => "",
        })
    }
    fn suffix(&self) -> String {
        self.complete_suffix().get(0..1).unwrap().to_string()
    }
    fn complete_suffix(&self) -> String {
        String::from(match self {
            Types::Void => "zero",
            Types::Char => "byte",
            Types::Int => "long",
            Types::Long => "quad",
        })
    }
    fn return_reg(&self) -> String {
        String::from(match self {
            Types::Void => unreachable!("doesnt have return register when returning void"),
            Types::Char => RETURN_REG[0],
            Types::Int => RETURN_REG[1],
            Types::Long => RETURN_REG[2],
        })
    }
}
impl Types {
    pub fn into_vec() -> Vec<TokenKind> {
        vec![
            TokenKind::Char,
            TokenKind::Int,
            TokenKind::Void,
            TokenKind::Long,
        ]
    }
    fn fmt(&self) -> &str {
        match self {
            Types::Void => "void",
            Types::Char => "char",
            Types::Int => "int",
            Types::Long => "long",
        }
    }

    fn max(&self) -> i64 {
        match self {
            Types::Void => unreachable!(),
            Types::Char => i8::MAX as i64,
            Types::Int => i32::MAX as i64,
            Types::Long => i64::MAX,
        }
    }
    fn min(&self) -> i64 {
        match self {
            Types::Void => unreachable!(),
            Types::Char => i8::MIN as i64,
            Types::Int => i32::MIN as i64,
            Types::Long => i64::MIN,
        }
    }
}

pub fn integer_type(n: i64) -> Types {
    if i32::try_from(n).is_ok() {
        Types::Int
    } else {
        Types::Long
    }
}