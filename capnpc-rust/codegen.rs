/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp;
use std::collections;
use std;
use schema_capnp;

pub fn tuple_option<T,U>(t : Option<T>, u : Option<U>) -> Option<(T,U)> {
    match (t, u) {
        (Some(t1), Some(u1)) => Some((t1,u1)),
        _ => None
    }
}

fn element_size_str (element_size : schema_capnp::ElementSize::Reader) -> &'static str {
    use schema_capnp::ElementSize::*;
    match element_size {
        Empty => "Void",
        Bit => "Bit",
        Byte => "Byte",
        TwoBytes => "TwoBytes",
        FourBytes => "FourBytes",
        EightBytes => "EightBytes",
        Pointer => "Pointer",
        InlineComposite => "InlineComposite"
    }
}

fn element_size (typ : schema_capnp::Type::WhichReader) -> schema_capnp::ElementSize::Reader {
    use schema_capnp::Type::*;
    use schema_capnp::ElementSize::*;
    match typ {
        Void(()) => Empty,
        Bool(()) => Bit,
        Int8(()) => Byte,
        Int16(()) => TwoBytes,
        Int32(()) => FourBytes,
        Int64(()) => EightBytes,
        Uint8(()) => Byte,
        Uint16(()) => TwoBytes,
        Uint32(()) => FourBytes,
        Uint64(()) => EightBytes,
        Float32(()) => FourBytes,
        Float64(()) => EightBytes,
        _ => fail!("not primitive")
    }
}

fn prim_type_str (typ : schema_capnp::Type::WhichReader) -> &'static str {
    use schema_capnp::Type::*;
    match typ {
        Void(()) => "()",
        Bool(()) => "bool",
        Int8(()) => "i8",
        Int16(()) => "i16",
        Int32(()) => "i32",
        Int64(()) => "i64",
        Uint8(()) => "u8",
        Uint16(()) => "u16",
        Uint32(()) => "u32",
        Uint64(()) => "u64",
        Float32(()) => "f32",
        Float64(()) => "f64",
        Enum(_) => "u16",
        _ => fail!("not primitive")
    }
}

#[allow(dead_code)]
fn camel_to_upper_case(s : &str) -> String {
    use std::ascii::*;
    let mut result_chars : Vec<char> = Vec::new();
    for c in s.chars() {
        assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}'", c));
        if std::char::is_uppercase(c) {
            result_chars.push('_');
        }
        result_chars.push((c as u8).to_ascii().to_uppercase().to_char());
    }
    return String::from_chars(result_chars.as_slice());
}

fn camel_to_snake_case(s : &str) -> String {
    use std::ascii::*;
    let mut result_chars : Vec<char> = Vec::new();
    for c in s.chars() {
        assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}', i.e. {}", c, c as uint));
        if std::char::is_uppercase(c) {
            result_chars.push('_');
        }
        result_chars.push((c as u8).to_ascii().to_lowercase().to_char());
    }
    return String::from_chars(result_chars.as_slice());
}

fn capitalize_first_letter(s : &str) -> String {
    use std::ascii::*;
    let mut result_chars : Vec<char> = Vec::new();
    for c in s.chars() { result_chars.push(c) }
    result_chars.as_mut_slice()[0] = (result_chars.as_slice()[0] as u8).to_ascii().to_uppercase().to_char();
    return String::from_chars(result_chars.as_slice());
}

#[test]
fn test_camel_to_upper_case() {
    assert_eq!(camel_to_upper_case("fooBar"), "FOO_BAR".to_string());
    assert_eq!(camel_to_upper_case("fooBarBaz"), "FOO_BAR_BAZ".to_string());
    assert_eq!(camel_to_upper_case("helloWorld"), "HELLO_WORLD".to_string());
}

#[test]
fn test_camel_to_snake_case() {
    assert_eq!(camel_to_snake_case("fooBar"), "foo_bar".to_string());
    assert_eq!(camel_to_snake_case("fooBarBaz"), "foo_bar_baz".to_string());
    assert_eq!(camel_to_snake_case("helloWorld"), "hello_world".to_string());
    assert_eq!(camel_to_snake_case("uint32Id"), "uint32_id".to_string());
}

#[deriving(PartialEq)]
enum FormattedText {
    Indent(Box<FormattedText>),
    Branch(Vec<FormattedText>),
    Line(String),
    BlankLine
}

fn to_lines(ft : &FormattedText, indent : uint) -> Vec<String> {
    match *ft {
        Indent (ref ft) => {
            return to_lines(&**ft, indent + 1);
        }
        Branch (ref fts) => {
            let mut result = Vec::new();
            for ft in fts.iter() {
                for line in to_lines(ft, indent).iter() {
                    result.push(line.clone());  // TODO there's probably a better way to do this.
                }
            }
            return result;
        }
        Line(ref s) => {
            let mut s1 = String::from_char(indent * 2, ' ');
            s1.push_str(s.as_slice());
            return vec!(s1.into_string());
        }
        BlankLine => return vec!("".to_string())
    }
}

fn stringify(ft : & FormattedText) -> String {
    let mut result = to_lines(ft, 0).connect("\n");
    result.push_str("\n");
    return result.into_string();
}

fn populate_scope_map(node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                      scope_map : &mut collections::hashmap::HashMap<u64, Vec<String>>,
                      scope_names : Vec<String>,
                      node_id : u64) {

    scope_map.insert(node_id, scope_names.clone());

    // unused nodes in imported files might be omitted from the node map
    let node_reader = match node_map.find(&node_id) { Some(node) => node, None => return (), };

    let nested_nodes = node_reader.get_nested_nodes();
    for ii in range(0, nested_nodes.size()) {
        let mut scope_names = scope_names.clone();
        scope_names.push(nested_nodes.get(ii).get_name().to_string());
        populate_scope_map(node_map, scope_map, scope_names, nested_nodes.get(ii).get_id());
    }

    match node_reader.which() {
        Some(schema_capnp::Node::Struct(struct_reader)) => {
            let fields = struct_reader.get_fields();
            for jj in range(0, fields.size()) {
                let field = fields.get(jj);
                match field.which() {
                    Some(schema_capnp::Field::Group(group)) => {
                        let name = capitalize_first_letter(field.get_name());
                        let mut scope_names = scope_names.clone();
                        scope_names.push(name);
                        populate_scope_map(node_map, scope_map, scope_names, group.get_type_id());
                    }
                    _ => {}
                }
            }
        }
        _ => {  }
    }
}

fn generate_import_statements() -> FormattedText {
    Branch(vec!(
        Line("use std;".to_string()),
        Line("use capnp::any_pointer;".to_string()),
        Line("use capnp::capability::{FromClientHook, FromTypelessPipeline};".to_string()),
        Line("use capnp::{text, data};".to_string()),
        Line("use capnp::layout;".to_string()),
        Line("use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};".to_string()),
        Line("use capnp::{primitive_list, enum_list, struct_list, text_list, data_list, list_list};".to_string()),
        Line("use capnp::list::ToU16;".to_string()),
    ))
}

fn list_list_type_param(scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
                        typ : schema_capnp::Type::Reader,
                        is_reader: bool,
                        lifetime_name: &str) -> String {
    use schema_capnp::Type;
    let module = if is_reader { "Reader" } else { "Builder" };
    match typ.which() {
        None => fail!("unsupported type"),
        Some(t) => {
            match t {
                Type::Void(()) | Type::Bool(()) | Type::Int8(()) |
                Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                Type::Uint64(()) | Type::Float32(()) | Type::Float64(()) => {
                    format!("primitive_list::{}<{}, {}>", module, lifetime_name, prim_type_str(t))
                }
                Type::Enum(en) => {
                    let the_mod = scope_map[en.get_type_id()].connect("::");
                    format!("enum_list::{}<{},{}::Reader>", module, lifetime_name, the_mod)
                }
                Type::Text(()) => {
                    format!("text_list::{}<{}>", module, lifetime_name)
                }
                Type::Data(()) => {
                    format!("data_list::{}<{}>", module, lifetime_name)
                }
                Type::Struct(st) => {
                    format!("struct_list::{}<{lifetime}, {}::{}<{lifetime}>>", module,
                            scope_map.get(&st.get_type_id()).connect("::"), module, lifetime = lifetime_name)
                }
                Type::List(t) => {
                    let inner = list_list_type_param(scope_map, t.get_element_type(), is_reader, lifetime_name);
                    format!("list_list::{}<{}, {}>", module, lifetime_name, inner)
                }
                Type::AnyPointer(()) => {
                    fail!("List(AnyPointer) is unsupported");
                }
                Type::Interface(_i) => {
                    fail!("unimplemented");
                }
            }
        }
    }
}

fn prim_default (value : &schema_capnp::Value::Reader) -> Option<String> {
    use schema_capnp::Value;
    match value.which() {
        Some(Value::Bool(false)) |
        Some(Value::Int8(0)) | Some(Value::Int16(0)) | Some(Value::Int32(0)) |
        Some(Value::Int64(0)) | Some(Value::Uint8(0)) | Some(Value::Uint16(0)) |
        Some(Value::Uint32(0)) | Some(Value::Uint64(0)) | Some(Value::Float32(0.0)) |
        Some(Value::Float64(0.0)) => None,

        Some(Value::Bool(true)) => Some(format!("true")),
        Some(Value::Int8(i)) => Some(i.to_string()),
        Some(Value::Int16(i)) => Some(i.to_string()),
        Some(Value::Int32(i)) => Some(i.to_string()),
        Some(Value::Int64(i)) => Some(i.to_string()),
        Some(Value::Uint8(i)) => Some(i.to_string()),
        Some(Value::Uint16(i)) => Some(i.to_string()),
        Some(Value::Uint32(i)) => Some(i.to_string()),
        Some(Value::Uint64(i)) => Some(i.to_string()),
        Some(Value::Float32(f)) => Some(format!("{}f32", f.to_string())),
        Some(Value::Float64(f)) => Some(format!("{}f64", f.to_string())),
        _ => {fail!()}
    }
}

fn getter_text (_node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
               scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
               field : &schema_capnp::Field::Reader,
               is_reader : bool)
    -> (String, FormattedText) {

    use schema_capnp::*;

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let the_mod = scope_map[group.get_type_id()].connect("::");
            if is_reader {
                return (format!("{}::Reader<'a>", the_mod),
                        Line("FromStructReader::new(self.reader)".to_string()));
            } else {
                return (format!("{}::Builder<'a>", the_mod),
                        Line("FromStructBuilder::new(self.builder)".to_string()));
            }
        }
        Some(Field::Slot(reg_field)) => {
            let offset = reg_field.get_offset() as uint;

            let member = if is_reader { "reader" } else { "builder" };
            let module = if is_reader { "Reader" } else { "Builder" };
            let module_with_var = if is_reader { "Reader<'a>" } else { "Builder<'a>" };

            match tuple_option(reg_field.get_type().which(), reg_field.get_default_value().which()) {
                Some((Type::Void(()), Value::Void(()))) => { return ("()".to_string(), Line("()".to_string()))}
                Some((Type::Bool(()), Value::Bool(b))) => {
                    if b {
                        return ("bool".to_string(), Line(format!("self.{}.get_bool_field_mask({}, true)",
                                                                member, offset)))
                    } else {
                        return ("bool".to_string(), Line(format!("self.{}.get_bool_field({})",
                                                                member, offset)))
                    }
                }
                Some((Type::Int8(()), Value::Int8(i))) => return common_case("i8", member, offset, i),
                Some((Type::Int16(()), Value::Int16(i))) => return common_case("i16", member, offset, i),
                Some((Type::Int32(()), Value::Int32(i))) => return common_case("i32", member, offset, i),
                Some((Type::Int64(()), Value::Int64(i))) => return common_case("i64", member, offset, i),
                Some((Type::Uint8(()), Value::Uint8(i))) => return common_case("u8", member, offset, i),
                Some((Type::Uint16(()), Value::Uint16(i))) => return common_case("u16", member, offset, i),
                Some((Type::Uint32(()), Value::Uint32(i))) => return common_case("u32", member, offset, i),
                Some((Type::Uint64(()), Value::Uint64(i))) => return common_case("u64", member, offset, i),
                Some((Type::Float32(()), Value::Float32(f))) => return common_case("f32", member, offset, f),
                Some((Type::Float64(()), Value::Float64(f))) => return common_case("f64", member, offset, f),
                Some((Type::Text(()), _)) => {
                    return (format!("text::{}", module_with_var),
                            Line(format!("self.{}.get_pointer_field({}).get_text(std::ptr::null(), 0)",
                                      member, offset)));
                }
                Some((Type::Data(()), _)) => {
                    return (format!("data::{}", module_with_var),
                            Line(format!("self.{}.get_pointer_field({}).get_data(std::ptr::null(), 0)",
                                      member, offset)));
                }
                Some((Type::List(ot1), _)) => {
                    match ot1.get_element_type().which() {
                        None => { fail!("unsupported type") }
                        Some(Type::Struct(st)) => {
                            let the_mod = scope_map[st.get_type_id()].connect("::");
                            if is_reader {
                                return (format!("struct_list::{}<'a,{}::{}<'a>>", module, the_mod, module),
                                        Line(format!("struct_list::{}::new(self.{}.get_pointer_field({}).get_list({}::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))",
                                                     module, member, offset, the_mod))
                                        );
                            } else {
                                return (format!("struct_list::{}<'a,{}::{}<'a>>", module, the_mod, module),
                                        Line(format!("struct_list::{}::new(self.{}.get_pointer_field({}).get_struct_list({}::STRUCT_SIZE, std::ptr::null()))",
                                                     module, member, offset, the_mod))
                                        );
                            }
                        }
                        Some(Type::Enum(e)) => {
                            let the_mod = scope_map[e.get_type_id()].connect("::");
                            let full_module_name = format!("{}::Reader", the_mod);
                            return (format!("enum_list::{}<'a,{}>",module,full_module_name),
                                    Line(format!("enum_list::{}::new(self.{}.get_pointer_field({}).get_list(layout::TwoBytes, std::ptr::null()))",
                                         module, member, offset)));
                        }
                        Some(Type::List(t1)) => {
                            let type_param = list_list_type_param(scope_map, t1.get_element_type(), is_reader, "'a");
                            return (format!("list_list::{}<'a,{}>", module, type_param),
                                    Line(format!("list_list::{}::new(self.{}.get_pointer_field({}).get_list(layout::Pointer, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Text(())) => {
                            return (format!("text_list::{}<'a>", module),
                                    Line(format!("text_list::{}::new(self.{}.get_pointer_field({}).get_list(layout::Pointer, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Data(())) => {
                            return (format!("data_list::{}<'a>", module),
                                    Line(format!("data_list::{}::new(self.{}.get_pointer_field({}).get_list(layout::Pointer, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Interface(_)) => {fail!("unimplemented") }
                        Some(Type::AnyPointer(())) => {fail!("List(AnyPointer) is unsupported")}
                        Some(prim_type) => {
                            let type_str = prim_type_str(prim_type);
                            let size_str = element_size_str(element_size(prim_type));
                            return
                                (format!("primitive_list::{}<'a,{}>", module, type_str),
                                 Line(format!("primitive_list::{}::new(self.{}.get_pointer_field({}).get_list(layout::{}, std::ptr::null()))",
                                           module, member, offset, size_str)))
                        }
                    }
                }
                Some((Type::Enum(en), _)) => {
                    let scope = &scope_map[en.get_type_id()];
                    let the_mod = scope.connect("::");
                    return
                        (format!("Option<{}::Reader>", the_mod), // Enums don't have builders.
                         Branch(vec!(
                            Line(format!("FromPrimitive::from_u16(self.{}.get_data_field::<u16>({}))",
                                        member, offset))
                              )));
                }
                Some((Type::Struct(st), _)) => {
                    let the_mod = scope_map[st.get_type_id()].connect("::");
                    let middle_arg = if is_reader {format!("")} else {format!("{}::STRUCT_SIZE,", the_mod)};
                    return (format!("{}::{}", the_mod, module_with_var),
                            Line(format!("FromStruct{}::new(self.{}.get_pointer_field({}).get_struct({} std::ptr::null()))",
                                      module, member, offset, middle_arg)))
                }
                Some((Type::Interface(interface), _)) => {
                    let the_mod = scope_map[interface.get_type_id()].connect("::");
                    return (format!("{}::Client", the_mod),
                            Line(format!("FromClientHook::new(self.{}.get_pointer_field({}).get_capability())",
                                         member, offset)));
                }
                Some((Type::AnyPointer(()), _)) => {
                    return (format!("any_pointer::{}<'a>", module),
                            Line(format!("any_pointer::{}::new(self.{}.get_pointer_field({}))",
                                         module, member, offset)))
                }
                None => {
                    // XXX should probably silently ignore, instead.
                    fail!("unrecognized type")
                }
                _ => {
                    fail!("default value was of wrong type");
                }

            }
        }
    }

    fn common_case<T:std::num::Zero + std::fmt::Show>(
        typ: &str, member : &str,
        offset: uint, default : T) -> (String, FormattedText) {
        let interior = if default.is_zero() {
            Line(format!("self.{}.get_data_field::<{}>({})",
                         member, typ, offset))
        } else {
            Line(format!("self.{}.get_data_field_mask::<{typ}>({}, {}{typ})",
                         member, offset, default, typ=typ))
        };
        return (typ.to_string(), interior);
    }


}

fn zero_fields_of_group(node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                        node_id : u64
                        ) -> FormattedText {
    use schema_capnp::{Node, Field, Type};
    match node_map[node_id].which() {
        Some(Node::Struct(st)) => {
            let mut result = Vec::new();
            if st.get_discriminant_count() != 0 {
                result.push(
                    Line(format!("self.builder.set_data_field::<u16>({}, 0);",
                                 st.get_discriminant_offset())));
            }
            let fields = st.get_fields();
            for ii in range(0, fields.size()) {
                match fields.get(ii).which() {
                    None => {fail!()}
                    Some(Field::Group(group)) => {
                        result.push(zero_fields_of_group(node_map, group.get_type_id()));
                    }
                    Some(Field::Slot(slot)) => {
                        match slot.get_type().which(){
                            Some(typ) => {
                                match typ {
                                    Type::Void(()) => {}
                                    Type::Bool(()) => {
                                        let line = Line(format!("self.builder.set_bool_field({}, false);",
                                                         slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                    Type::Int8(()) |
                                    Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                                    Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                                    Type::Uint64(()) | Type::Float32(()) | Type::Float64(()) |
                                    Type::Enum(_) => {
                                        let line = Line(format!("self.builder.set_data_field::<{}>({}, 0);",
                                                         prim_type_str(typ),
                                                         slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                    Type::Struct(_) | Type::List(_) | Type::Text(()) | Type::Data(()) |
                                    Type::AnyPointer(()) |
                                    Type::Interface(_) // Is this the right thing to do for interfaces?
                                        => {
                                        let line = Line(format!("self.builder.get_pointer_field({}).clear();",
                                                                slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                }
                            }
                            None => {fail!()}
                        }
                    }
                }
            }
            return Branch(result);
        }
        _ => { fail!("expected a struct") }
    }
}

fn generate_setter(node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
                  discriminant_offset : u32,
                  styled_name : &str,
                  field :&schema_capnp::Field::Reader) -> FormattedText {

    use schema_capnp::*;

    let mut setter_interior = Vec::new();
    let mut setter_param = "value".to_string();
    let mut initter_interior = Vec::new();
    let mut initter_params = Vec::new();

    let discriminant_value = field.get_discriminant_value();
    if discriminant_value != Field::NO_DISCRIMINANT {
        setter_interior.push(
            Line(format!("self.builder.set_data_field::<u16>({}, {});",
                         discriminant_offset as uint,
                         discriminant_value as uint)));
        initter_interior.push(
            Line(format!("self.builder.set_data_field::<u16>({}, {});",
                         discriminant_offset as uint,
                         discriminant_value as uint)));
    }

    let mut setter_lifetime_param = "";

    let (maybe_reader_type, maybe_builder_type) : (Option<String>, Option<String>) = match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let scope = &scope_map[group.get_type_id()];
            let the_mod = scope.connect("::");

            initter_interior.push(zero_fields_of_group(node_map, group.get_type_id()));

            initter_interior.push(Line(format!("FromStructBuilder::new(self.builder)")));

            (None, Some(format!("{}::Builder<'a>", the_mod)))
        }
        Some(Field::Slot(reg_field)) => {
            fn common_case (typ: &str, offset : uint, reg_field : Field::Slot::Reader,
                            setter_interior : &mut Vec<FormattedText> ) -> (Option<String>, Option<String>) {
                match prim_default(&reg_field.get_default_value()) {
                    None => {
                        setter_interior.push(Line(format!("self.builder.set_data_field::<{}>({}, value);",
                                                          typ, offset)));
                    }
                    Some(s) => {
                        setter_interior.push(
                            Line(format!("self.builder.set_data_field_mask::<{}>({}, value, {});",
                                         typ, offset, s)));
                    }
                }
                (Some(typ.to_string()), None)
            };


            let offset = reg_field.get_offset() as uint;

            match reg_field.get_type().which() {
                Some(Type::Void(())) => {
                    setter_param = "_value".to_string();
                    (Some("()".to_string()), None)
                }
                Some(Type::Bool(())) => {
                    match prim_default(&reg_field.get_default_value()) {
                        None => {
                            setter_interior.push(Line(format!("self.builder.set_bool_field({}, value);", offset)));
                        }
                        Some(s) => {
                            setter_interior.push(
                                Line(format!("self.builder.set_bool_field_mask({}, value, {});", offset, s)));
                        }
                    }
                    (Some("bool".to_string()), None)
                }
                Some(Type::Int8(())) => common_case("i8", offset, reg_field, &mut setter_interior),
                Some(Type::Int16(())) => common_case("i16", offset, reg_field, &mut setter_interior),
                Some(Type::Int32(())) => common_case("i32", offset, reg_field, &mut setter_interior),
                Some(Type::Int64(())) => common_case("i64", offset, reg_field, &mut setter_interior),
                Some(Type::Uint8(())) => common_case("u8", offset, reg_field, &mut setter_interior),
                Some(Type::Uint16(())) => common_case("u16", offset, reg_field, &mut setter_interior),
                Some(Type::Uint32(())) => common_case("u32", offset, reg_field, &mut setter_interior),
                Some(Type::Uint64(())) => common_case("u64", offset, reg_field, &mut setter_interior),
                Some(Type::Float32(())) => common_case("f32", offset, reg_field, &mut setter_interior),
                Some(Type::Float64(())) => common_case("f64", offset, reg_field, &mut setter_interior),
                Some(Type::Text(())) => {
                    setter_interior.push(Line(format!("self.builder.get_pointer_field({}).set_text(value);",
                                                      offset)));
                    initter_interior.push(Line(format!("self.builder.get_pointer_field({}).init_text(size)",
                                                       offset)));
                    initter_params.push("size : uint");
                    (Some("text::Reader".to_string()), Some("text::Builder<'a>".to_string()))
                }
                Some(Type::Data(())) => {
                    setter_interior.push(Line(format!("self.builder.get_pointer_field({}).set_data(value);",
                                                      offset)));
                    initter_interior.push(Line(format!("self.builder.get_pointer_field({}).init_data(size)",
                                                       offset)));
                    initter_params.push("size : uint");
                    (Some("data::Reader".to_string()), Some("data::Builder<'a>".to_string()))
                }
                Some(Type::List(ot1)) => {
                    setter_interior.push(
                        Line(format!("self.builder.get_pointer_field({}).set_list(&value.reader)",
                                     offset)));

                    initter_params.push("size : uint");
                    match ot1.get_element_type().which() {
                        None => fail!("unsupported type"),
                        Some(t1) => {
                            match t1 {
                                Type::Void(()) | Type::Bool(()) | Type::Int8(()) |
                                Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                                Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                                Type::Uint64(()) | Type::Float32(()) | Type::Float64(()) => {

                                    let type_str = prim_type_str(t1);
                                    let size_str = element_size_str(element_size(t1));

                                    initter_interior.push(Line(format!("primitive_list::Builder::<'a,{}>::new(",
                                                               type_str)));
                                    initter_interior.push(
                                        Indent(box Line(format!("self.builder.get_pointer_field({}).init_list(layout::{},size)",
                                                                offset, size_str))));
                                    initter_interior.push(Line(")".to_string()));

                                    (Some(format!("primitive_list::Reader<'a,{}>", type_str)),
                                     Some(format!("primitive_list::Builder<'a,{}>", type_str)))
                                }
                                Type::Enum(e) => {
                                    let id = e.get_type_id();
                                    let scope = &scope_map[id];
                                    let the_mod = scope.connect("::");
                                    let type_str = format!("{}::Reader", the_mod);
                                    initter_interior.push(Line(format!("enum_list::Builder::<'a, {}>::new(",
                                                            type_str)));
                                    initter_interior.push(
                                        Indent(
                                            box Line(
                                                format!("self.builder.get_pointer_field({}).init_list(layout::TwoBytes,size)",
                                                     offset))));
                                    initter_interior.push(Line(")".to_string()));
                                    (Some(format!("enum_list::Reader<'a,{}>", type_str)),
                                     Some(format!("enum_list::Builder<'a,{}>", type_str)))
                                }
                                Type::Struct(st) => {
                                    let id = st.get_type_id();
                                    let scope = &scope_map[id];
                                    let the_mod = scope.connect("::");

                                    initter_interior.push(Line(format!("struct_list::Builder::<'a, {}::Builder<'a>>::new(", the_mod)));
                                    initter_interior.push(
                                       Indent(
                                          box Line(
                                             format!("self.builder.get_pointer_field({}).init_struct_list(size, {}::STRUCT_SIZE))",
                                                  offset, the_mod))));

                                    (Some(format!("struct_list::Reader<'a,{}::Reader<'a>>", the_mod)),
                                     Some(format!("struct_list::Builder<'a,{}::Builder<'a>>", the_mod)))
                                }
                                Type::Text(()) => {
                                    initter_interior.push(
                                        Line(format!("text_list::Builder::<'a>::new(self.builder.get_pointer_field({}).init_list(layout::Pointer, size))", offset)));

                                    (Some(format!("text_list::Reader")),
                                     Some(format!("text_list::Builder<'a>")))
                                }
                                Type::Data(()) => {
                                    initter_interior.push(
                                        Line(format!("data_list::Builder::<'a>::new(self.builder.get_pointer_field({}).init_list(layout::Pointer, size))", offset)));

                                    (Some(format!("data_list::Reader")),
                                     Some(format!("data_list::Builder<'a>")))
                                }
                                Type::List(t1) => {
                                    let type_param = list_list_type_param(scope_map, t1.get_element_type(),
                                                                          false, "'a");
                                    initter_interior.push(
                                        Line(format!("list_list::Builder::<'a,{}>::new(self.builder.get_pointer_field({}).init_list(layout::Pointer,size))",
                                                     type_param, offset)));

                                    setter_lifetime_param = "<'b>";

                                    (Some(format!("list_list::Reader<'b, {}>",
                                             list_list_type_param(scope_map, t1.get_element_type(), true, "'b"))),
                                     Some(format!("list_list::Builder<'a, {}>", type_param)))
                                }
                                Type::AnyPointer(()) => {fail!("List(AnyPointer) not supported")}
                                Type::Interface(_) => { fail!("unimplemented") }
                            }
                        }
                    }
                }
                Some(Type::Enum(e)) => {
                    let id = e.get_type_id();
                    let the_mod = scope_map[id].connect("::");
                    setter_interior.push(
                        Line(format!("self.builder.set_data_field::<u16>({}, value as u16)",
                                     offset)));
                    (Some(format!("{}::Reader", the_mod)), None)
                }
                Some(Type::Struct(st)) => {
                    let the_mod = scope_map[st.get_type_id()].connect("::");
                    setter_interior.push(
                        Line(format!("self.builder.get_pointer_field({}).set_struct(&value.struct_reader())", offset)));
                    initter_interior.push(
                      Line(format!("FromStructBuilder::new(self.builder.get_pointer_field({}).init_struct({}::STRUCT_SIZE))",
                                   offset, the_mod)));
                    (Some(format!("{}::Reader", the_mod)), Some(format!("{}::Builder<'a>", the_mod)))
                }
                Some(Type::Interface(interface)) => {
                    let the_mod = scope_map[interface.get_type_id()].connect("::");
                    setter_interior.push(
                        Line(format!("self.builder.get_pointer_field({}).set_capability(value.client.hook);",
                                     offset)));
                    (Some(format!("{}::Client",the_mod)), None)
                }
                Some(Type::AnyPointer(())) => {
                    initter_interior.push(Line(format!("let result = any_pointer::Builder::new(self.builder.get_pointer_field({}));",
                                               offset)));
                    initter_interior.push(Line("result.clear();".to_string()));
                    initter_interior.push(Line("result".to_string()));
                    (None, Some("any_pointer::Builder<'a>".to_string()))
                }
                None => { fail!("unrecognized type") }
            }
        }
    };
    let mut result = Vec::new();
    match maybe_reader_type {
        Some(reader_type) => {
            result.push(Line("#[inline]".to_string()));
            result.push(Line(format!("pub fn set_{}{}(&self, {} : {}) {{",
                                     styled_name, setter_lifetime_param, setter_param, reader_type)));
            result.push(Indent(box Branch(setter_interior)));
            result.push(Line("}".to_string()));
        }
        None => {}
    }
    match maybe_builder_type {
        Some(builder_type) => {
            result.push(Line("#[inline]".to_string()));
            let args = initter_params.connect(", ");
            result.push(Line(format!("pub fn init_{}(&self, {}) -> {} {{",
                                     styled_name, args, builder_type)));
            result.push(Indent(box Branch(initter_interior)));
            result.push(Line("}".to_string()));
        }
        None => {}
    }
    return Branch(result);
}


// return (the 'Which' enum, the 'which()' accessor, typedef)
fn generate_union(node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
                  discriminant_offset : u32,
                  fields : &[schema_capnp::Field::Reader],
                  is_reader : bool)
                  -> (FormattedText, FormattedText, FormattedText)
{
    use schema_capnp::*;

    fn new_ty_param(ty_params : &mut Vec<String>) -> String {
        let result = format!("A{}", ty_params.len());
        ty_params.push(result.clone());
        result
    }

    let mut getter_interior = Vec::new();
    let mut interior = Vec::new();
    let mut enum_interior = Vec::new();

    let mut ty_params = Vec::new();
    let mut ty_args = Vec::new();

    let doffset = discriminant_offset as uint;

    for field in fields.iter() {

        let dvalue = field.get_discriminant_value() as uint;

        let field_name = field.get_name();
        let enumerant_name = capitalize_first_letter(field_name);

        let (ty, get) = getter_text(node_map, scope_map, field, is_reader);

        getter_interior.push(Branch(vec!(
                    Line(format!("{} => {{", dvalue)),
                    Indent(box Line(format!("return std::option::Some({}(", enumerant_name.clone()))),
                    Indent(box Indent(box get)),
                    Indent(box Line("));".to_string())),
                    Line("}".to_string())
                )));

        let ty1 = match field.which() {
            Some(Field::Group(_)) => {
                ty_args.push(ty);
                new_ty_param(&mut ty_params)
            }
            Some(Field::Slot(reg_field)) => {
                match reg_field.get_type().which() {
                    Some(Type::Text(())) | Some(Type::Data(())) |
                    Some(Type::List(_)) | Some(Type::Struct(_)) |
                    Some(Type::AnyPointer(())) => {
                        ty_args.push(ty);
                        new_ty_param(&mut ty_params)
                    }
                    _ => ty
                }
            }
            _ => ty
        };

        enum_interior.push(Line(format!("{}({}),", enumerant_name, ty1)));
    }

    let enum_name = format!("Which{}",
                            if ty_params.len() > 0 { format!("<'a,{}>",ty_params.connect(",")) }
                            else {"".to_string()} );


    getter_interior.push(Line("_ => return std::option::None".to_string()));

    interior.push(
        Branch(vec!(Line(format!("pub enum {} {{", enum_name)),
                    Indent(box Branch(enum_interior)),
                    Line("}".to_string()))));


    let result = if is_reader {
        Branch(interior)
    } else {
        Branch(vec!(Line("pub mod Which {".to_string()),
                    Indent(box generate_import_statements()),
                    BlankLine,
                    Indent(box Branch(interior)),
                    Line("}".to_string())))
    };

    let field_name = if is_reader { "reader" } else { "builder" };

    let concrete_type =
            format!("Which{}{}",
                    if is_reader {"Reader"} else {"Builder"},
                    if ty_params.len() > 0 {"<'a>"} else {""});

    let typedef = Line(format!("pub type {} = Which{};",
                               concrete_type,
                               if ty_args.len() > 0 {format!("<'a,{}>",ty_args.connect(","))} else {"".to_string()}));

    let getter_result =
        Branch(vec!(Line("#[inline]".to_string()),
                    Line(format!("pub fn which(&self) -> std::option::Option<{}> {{",
                                 concrete_type)),
                    Indent(box Branch(vec!(
                        Line(format!("match self.{}.get_data_field::<u16>({}) {{", field_name, doffset)),
                        Indent(box Branch(getter_interior)),
                        Line("}".to_string())))),
                    Line("}".to_string())));

    // TODO set_which() for builders?

    return (result, getter_result, typedef);
}

fn generate_haser(discriminant_offset : u32,
                  styled_name : &str,
                  field :&schema_capnp::Field::Reader,
                  is_reader : bool) -> FormattedText {

    use schema_capnp::*;

    let mut result = Vec::new();
    let mut interior = Vec::new();
    let member = if is_reader { "reader" } else { "builder" };

    let discriminant_value = field.get_discriminant_value();
    if discriminant_value != Field::NO_DISCRIMINANT {
       interior.push(
            Line(format!("if self.{}.get_data_field::<u16>({}) != {} {{ return false; }}",
                         member,
                         discriminant_offset as uint,
                         discriminant_value as uint)));
    }
    match field.which() {
        None | Some(Field::Group(_)) => {},
        Some(Field::Slot(reg_field)) => {
            match reg_field.get_type().which() {
                Some(Type::Text(())) | Some(Type::Data(())) |
                    Some(Type::List(_)) | Some(Type::Struct(_)) |
                    Some(Type::AnyPointer(())) => {
                    interior.push(
                        Line(format!("!self.{}.get_pointer_field({}).is_null()",
                                     member, reg_field.get_offset())));
                    result.push(
                        Line(format!("pub fn has_{}(&self) -> bool {{", styled_name)));
                    result.push(
                        Indent(box Branch(interior)));
                    result.push(Line("}".to_string()));
                }
                _ => {}
            }
        }
    }

    Branch(result)
}

fn generate_pipeline_getter(_node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                            scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
                            field : schema_capnp::Field::Reader) -> FormattedText {
    use schema_capnp::{Field, Type};

    let name = field.get_name();

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let the_mod = scope_map[group.get_type_id()].connect("::");
            return Branch(vec!(Line(format!("pub fn get_{}(&self) -> {}::Pipeline {{",
                                            camel_to_snake_case(name),
                                            the_mod)),
                               Indent(box Line("FromTypelessPipeline::new(self._typeless.noop())".to_string())),
                               Line("}".to_string())));
        }
        Some(Field::Slot(reg_field)) => {
            match reg_field.get_type().which() {
                None => fail!("unrecognized type"),
                Some(Type::Struct(st)) => {
                    let the_mod = scope_map[st.get_type_id()].connect("::");
                    return Branch(vec!(
                        Line(format!("pub fn get_{}(&self) -> {}::Pipeline {{",
                                     camel_to_snake_case(name),
                                     the_mod)),
                        Indent(box Line(
                            format!("FromTypelessPipeline::new(self._typeless.get_pointer_field({}))",
                                    reg_field.get_offset()))),
                        Line("}".to_string())));
                }
                Some(Type::Interface(interface)) => {
                    let the_mod = scope_map[interface.get_type_id()].connect("::");
                    return Branch(vec!(
                        Line(format!("pub fn get_{}(&self) -> {}::Client {{",
                                     camel_to_snake_case(name),
                                     the_mod)),
                        Indent(box Line(
                            format!("FromClientHook::new(self._typeless.get_pointer_field({}).as_cap())",
                                    reg_field.get_offset()))),
                        Line("}".to_string())));
                }
                _ => {
                    return Branch(Vec::new());
                }
            }
        }
    }
}


fn generate_node(node_map : &collections::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                 scope_map : &collections::hashmap::HashMap<u64, Vec<String>>,
                 node_id : u64,
                 node_name: &str) -> FormattedText {
    use schema_capnp::*;

    let mut output: Vec<FormattedText> = Vec::new();
    let mut nested_output: Vec<FormattedText> = Vec::new();

    let node_reader = &node_map[node_id];
    let nested_nodes = node_reader.get_nested_nodes();
    for ii in range(0, nested_nodes.size()) {
        let id = nested_nodes.get(ii).get_id();
        nested_output.push(generate_node(node_map, scope_map,
                                         id, scope_map[id].last().unwrap().as_slice()));
    }

    match node_reader.which() {

        Some(Node::File(())) => {
            output.push(Branch(nested_output));
        }

        Some(Node::Struct(struct_reader)) => {
            output.push(BlankLine);
            output.push(Line(format!("pub mod {} {{", node_name)));

            let mut preamble = Vec::new();
            let mut builder_members = Vec::new();
            let mut reader_members = Vec::new();
            let mut union_fields = Vec::new();
            let mut which_enums = Vec::new();
            let mut pipeline_impl_interior = Vec::new();

            let data_size = struct_reader.get_data_word_count();
            let pointer_size = struct_reader.get_pointer_count();
            let preferred_list_encoding =
                  match struct_reader.get_preferred_list_encoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };
            let is_group = struct_reader.get_is_group();
            let discriminant_count = struct_reader.get_discriminant_count();
            let discriminant_offset = struct_reader.get_discriminant_offset();

            preamble.push(generate_import_statements());
            preamble.push(BlankLine);


            if !is_group {
                preamble.push(Line("pub static STRUCT_SIZE : layout::StructSize =".to_string()));
                preamble.push(
                   Indent(
                      box Line(
                        format!("layout::StructSize {{ data : {}, pointers : {}, preferred_list_encoding : layout::{}}};",
                             data_size as uint, pointer_size as uint,
                             element_size_str(preferred_list_encoding)))));
                preamble.push(BlankLine);

                preamble.push(BlankLine);
            }

            let fields = struct_reader.get_fields();
            for ii in range(0, fields.size()) {
                let field = fields.get(ii);
                let name = field.get_name();
                let styled_name = camel_to_snake_case(name);

                let discriminant_value = field.get_discriminant_value();
                let is_union_field = discriminant_value != Field::NO_DISCRIMINANT;

                if !is_union_field {
                    pipeline_impl_interior.push(generate_pipeline_getter(node_map, scope_map, field));
                    let (ty, get) = getter_text(node_map, scope_map, &field, true);
                    reader_members.push(
                        Branch(vec!(
                            Line("#[inline]".to_string()),
                            Line(format!("pub fn get_{}(&self) -> {} {{", styled_name, ty)),
                            Indent(box get),
                            Line("}".to_string()))));

                    let (ty_b, get_b) = getter_text(node_map, scope_map, &field, false);

                    builder_members.push(
                        Branch(vec!(
                            Line("#[inline]".to_string()),
                            Line(format!("pub fn get_{}(&self) -> {} {{", styled_name, ty_b)),
                            Indent(box get_b),
                            Line("}".to_string()))));

                } else {
                    union_fields.push(field);
                }

                builder_members.push(generate_setter(node_map, scope_map,
                                                    discriminant_offset,
                                                    styled_name.as_slice(), &field));

                reader_members.push(generate_haser(discriminant_offset, styled_name.as_slice(), &field, true));
                builder_members.push(generate_haser(discriminant_offset, styled_name.as_slice(), &field, false));

                match field.which() {
                    Some(Field::Group(group)) => {
                        let id = group.get_type_id();
                        let text = generate_node(node_map, scope_map,
                                                 id, scope_map[id].last().unwrap().as_slice());
                        nested_output.push(text);
                    }
                    _ => { }
                }

            }

            if discriminant_count > 0 {
                let (which_enums1, union_getter, typedef) =
                    generate_union(node_map, scope_map,
                                   discriminant_offset, union_fields.as_slice(), true);
                which_enums.push(which_enums1);
                which_enums.push(typedef);
                reader_members.push(union_getter);

                let (_, union_getter, typedef) =
                    generate_union(node_map, scope_map,
                                   discriminant_offset, union_fields.as_slice(), false);
                which_enums.push(typedef);
                builder_members.push(union_getter);
            }

            let builder_struct_size =
                if is_group { Branch(Vec::new()) }
                else {
                    Branch(vec!(
                        Line("impl <'a> layout::HasStructSize for Builder<'a> {".to_string()),
                        Indent(box Branch(vec!(Line("#[inline]".to_string()),
                                            Line("fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }".to_string())))),
                       Line("}".to_string())))
            };

            let accessors = vec!(
                Branch(preamble),
                Line("pub struct Reader<'a> { reader : layout::StructReader<'a> }".to_string()),
                BlankLine,
                Line("impl <'a> layout::FromStructReader<'a> for Reader<'a> {".to_string()),
                Indent(
                    box Branch(vec!(
                        Line("fn new(reader: layout::StructReader<'a>) -> Reader<'a> {".to_string()),
                        Indent(box Line("Reader { reader : reader }".to_string())),
                        Line("}".to_string())))),
                Line("}".to_string()),
                BlankLine,
                Line("impl <'a> layout::ToStructReader<'a> for Reader<'a> {".to_string()),
                Indent(box Line("fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }".to_string())),
                Line("}".to_string()),
                BlankLine,
                Line("impl <'a> Reader<'a> {".to_string()),
                Indent(box Branch(reader_members)),
                Line("}".to_string()),
                BlankLine,
                Line("pub struct Builder<'a> { builder : layout::StructBuilder<'a> }".to_string()),
                builder_struct_size,
                Line("impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {".to_string()),
                Indent(
                    box Branch(vec!(
                        Line("fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {".to_string()),
                        Indent(box Line("Builder { builder : builder }".to_string())),
                        Line("}".to_string())))),
                Line("}".to_string()),

                Line("impl <'a> Builder<'a> {".to_string()),
                Indent(
                    box Branch(vec!(
                        Line("pub fn as_reader(&self) -> Reader<'a> {".to_string()),
                        Indent(box Line("FromStructReader::new(self.builder.as_reader())".to_string())),
                        Line("}".to_string())))),
                Indent(box Branch(builder_members)),
                Line("}".to_string()),
                BlankLine,
                Line("pub struct Pipeline { _typeless : any_pointer::Pipeline }".to_string()),
                Line("impl FromTypelessPipeline for Pipeline {".to_string()),
                Indent(
                    box Branch(vec!(
                        Line("fn new(typeless : any_pointer::Pipeline) -> Pipeline {".to_string()),
                        Indent(box Line("Pipeline { _typeless : typeless }".to_string())),
                        Line("}".to_string())))),
                Line("}".to_string()),
                Line("impl Pipeline {".to_string()),
                Indent(box Branch(pipeline_impl_interior)),
                Line("}".to_string()),
                );

            output.push(Indent(box Branch(vec!(Branch(accessors),
                                            Branch(which_enums),
                                            Branch(nested_output)))));
            output.push(Line("}".to_string()));

        }

        Some(Node::Enum(enum_reader)) => {
            let names = &scope_map[node_id];
            output.push(BlankLine);
            output.push(Line(format!("pub mod {} {{", *names.last().unwrap())));

            output.push(Indent(box Line("use capnp::list::{ToU16};".to_string())));
            output.push(BlankLine);

            let mut members = Vec::new();
            let enumerants = enum_reader.get_enumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants.get(ii);
                members.push(
                    Line(format!("{} = {},", capitalize_first_letter(enumerant.get_name()),
                              ii)));
            }

            output.push(Indent(box Branch(vec!(
                Line("#[repr(u16)]".to_string()),
                Line("#[deriving(FromPrimitive)]".to_string()),
                Line("#[deriving(PartialEq)]".to_string()),
                Line("pub enum Reader {".to_string()),
                Indent(box Branch(members)),
                Line("}".to_string())))));

            output.push(
                Indent(
                    box Branch(vec!(
                        Line("impl ToU16 for Reader {".to_string()),
                        Indent(box Line("#[inline]".to_string())),
                        Indent(
                            box Line("fn to_u16(self) -> u16 { self as u16 }".to_string())),
                        Line("}".to_string())))));

            output.push(Line("}".to_string()));
        }

        Some(Node::Interface(interface)) => {
            let names = &scope_map[node_id];
            let mut client_impl_interior = Vec::new();
            let mut server_interior = Vec::new();
            let mut mod_interior = Vec::new();
            let mut dispatch_arms = Vec::new();

            mod_interior.push(Line ("#![allow(unused_variable)]".to_string()));
            mod_interior.push(Line("use capnp::any_pointer;".to_string()));
            mod_interior.push(
                Line("use capnp::capability::{ClientHook, FromClientHook, FromServer, Request, ServerHook};".to_string()));
            mod_interior.push(Line("use capnp::capability;".to_string()));
            mod_interior.push(BlankLine);

            let methods = interface.get_methods();
            for ordinal in range(0, methods.size()) {
                let method = methods.get(ordinal);
                let name = method.get_name();

                method.get_code_order();
                let params_id = method.get_param_struct_type();
                let params_node = &node_map[params_id];
                let params_name = if params_node.get_scope_id() == 0 {
                    let params_name = format!("{}Params", capitalize_first_letter(name));

                    nested_output.push(generate_node(node_map, scope_map,
                                                     params_id, params_name.as_slice()));
                    params_name
                } else {
                    scope_map[params_node.get_id()].connect("::")
                };

                let results_id = method.get_result_struct_type();
                let results_node = node_map[results_id];
                let results_name = if results_node.get_scope_id() == 0 {
                    let results_name = format!("{}Results", capitalize_first_letter(name));
                    nested_output.push(generate_node(node_map, scope_map,
                                                     results_id, results_name.as_slice() ));
                    results_name
                } else {
                    scope_map[results_node.get_id()].connect("::")
                };

                dispatch_arms.push(
                    Line(format!(
                            "{} => server.{}(capability::internal_get_typed_context(context)),",
                            ordinal, camel_to_snake_case(name))));

                mod_interior.push(
                    Line(format!(
                            "pub type {}Context<'a> = capability::CallContext<{}::Reader<'a>, {}::Builder<'a>>;",
                            capitalize_first_letter(name), params_name, results_name)));
                server_interior.push(
                    Line(format!(
                            "fn {}<'a>(&mut self, {}Context<'a>);",
                            camel_to_snake_case(name), capitalize_first_letter(name)
                            )));

                client_impl_interior.push(
                    Line(format!("pub fn {}_request<'a>(&self) -> Request<{}::Builder<'a>,{}::Reader<'a>,{}::Pipeline> {{",
                                 camel_to_snake_case(name), params_name, results_name, results_name)));

                client_impl_interior.push(Indent(
                        box Line(format!("self.client.new_call(0x{:x}, {}, None)", node_id, ordinal))));
                client_impl_interior.push(Line("}".to_string()));

                method.get_annotations();
            }

            let mut base_dispatch_arms = Vec::new();
            let server_base = {
                let mut base_traits = Vec::new();
                let extends = interface.get_extends();
                for ii in range(0, extends.size()) {
                    let base_id = extends.get(ii);
                    let the_mod = scope_map[base_id].connect("::");
                    base_dispatch_arms.push(
                        Line(format!(
                                "0x{:x} => {}::ServerDispatch::<T>::dispatch_call_internal(&mut *self.server, method_id, context),",
                                base_id, the_mod)));
                    base_traits.push(format!("{}::Server", the_mod));
                }
                if extends.size() > 0 { format!(": {}", base_traits.as_slice().connect(" + ")) }
                else { "".to_string() }
            };


            mod_interior.push(BlankLine);
            mod_interior.push(Line("pub struct Client{ pub client : capability::Client }".to_string()));
            mod_interior.push(
                Branch(vec!(
                    Line("impl FromClientHook for Client {".to_string()),
                    Indent(box Line("fn new(hook : Box<ClientHook+Send>) -> Client {".to_string())),
                    Indent(box Indent(box Line("Client { client : capability::Client::new(hook) }".to_string()))),
                    Indent(box Line("}".to_string())),
                    Line("}".to_string()))));


            mod_interior.push(
                Branch(vec!(
                    Line("impl <T:ServerHook, U : Server + Send> FromServer<T,U> for Client {".to_string()),
                    Indent(box Branch( vec!(
                        Line("fn new(_hook : Option<T>, server : Box<U>) -> Client {".to_string()),
                        Indent(
                            box Line("Client { client : ServerHook::new_client(None::<T>, box ServerDispatch { server : server})}".to_string())),
                        Line("}".to_string())))),
                    Line("}".to_string()))));


            mod_interior.push(
                    Branch(vec!(
                        Line("impl Clone for Client {".to_string()),
                        Indent(box Line("fn clone(&self) -> Client {".to_string())),
                        Indent(box Indent(box Line("Client { client : capability::Client::new(self.client.hook.copy()) }".to_string()))),
                        Indent(box Line("}".to_string())),
                        Line("}".to_string()))));


            mod_interior.push(
                Branch(vec!(Line("impl Client {".to_string()),
                            Indent(box Branch(client_impl_interior)),
                            Line("}".to_string()))));

            mod_interior.push(Branch(vec!(Line(format!("pub trait Server {} {{", server_base)),
                                          Indent(box Branch(server_interior)),
                                          Line("}".to_string()))));

            mod_interior.push(Branch(vec!(Line("pub struct ServerDispatch<T> {".to_string()),
                                          Indent(box Line("pub server : Box<T>,".to_string())),
                                          Line("}".to_string()))));

            mod_interior.push(
                Branch(vec!(
                    Line("impl <T : Server> capability::Server for ServerDispatch<T> {".to_string()),
                    Indent(box Line("fn dispatch_call(&mut self, interface_id : u64, method_id : u16, context : capability::CallContext<any_pointer::Reader, any_pointer::Builder>) {".to_string())),
                    Indent(box Indent(box Line("match interface_id {".to_string()))),
                    Indent(box Indent(box Indent(
                        box Line(format!("0x{:x} => ServerDispatch::<T>::dispatch_call_internal(&mut *self.server, method_id, context),",
                                                     node_id))))),
                    Indent(box Indent(box Indent(box Branch(base_dispatch_arms)))),
                    Indent(box Indent(box Indent(box Line("_ => {}".to_string())))),
                    Indent(box Indent(box Line("}".to_string()))),
                    Indent(box Line("}".to_string())),
                    Line("}".to_string()))));

            mod_interior.push(
                Branch(vec!(
                    Line("impl <T : Server> ServerDispatch<T> {".to_string()),
                    Indent(box Line("pub fn dispatch_call_internal(server :&mut T, method_id : u16, context : capability::CallContext<any_pointer::Reader, any_pointer::Builder>) {".to_string())),
                    Indent(box Indent(box Line("match method_id {".to_string()))),
                    Indent(box Indent(box Indent(box Branch(dispatch_arms)))),
                    Indent(box Indent(box Indent(box Line("_ => {}".to_string())))),
                    Indent(box Indent(box Line("}".to_string()))),
                    Indent(box Line("}".to_string())),
                    Line("}".to_string()))));


            mod_interior.push(Branch(vec!(Branch(nested_output))));


            output.push(BlankLine);
            output.push(Line(format!("pub mod {} {{", *names.last().unwrap())));
            output.push(Indent(box Branch(mod_interior)));
            output.push(Line("}".to_string()));
        }

        Some(Node::Const(c)) => {
            let names = &scope_map[node_id];
            let styled_name = camel_to_upper_case(names.last().unwrap().as_slice());

            let (typ, txt) = match tuple_option(c.get_type().which(), c.get_value().which()) {
                Some((Type::Void(()), Value::Void(()))) => ("()".to_string(), "()".to_string()),
                Some((Type::Bool(()), Value::Bool(b))) => ("bool".to_string(), b.to_string()),
                Some((Type::Int8(()), Value::Int8(i))) => ("i8".to_string(), i.to_string()),
                Some((Type::Int16(()), Value::Int16(i))) => ("i16".to_string(), i.to_string()),
                Some((Type::Int32(()), Value::Int32(i))) => ("i32".to_string(), i.to_string()),
                Some((Type::Int64(()), Value::Int64(i))) => ("i64".to_string(), i.to_string()),
                Some((Type::Uint8(()), Value::Uint8(i))) => ("u8".to_string(), i.to_string()),
                Some((Type::Uint16(()), Value::Uint16(i))) => ("u16".to_string(), i.to_string()),
                Some((Type::Uint32(()), Value::Uint32(i))) => ("u32".to_string(), i.to_string()),
                Some((Type::Uint64(()), Value::Uint64(i))) => ("u64".to_string(), i.to_string()),

                // float string formatting appears to be a bit broken currently, in Rust.
                Some((Type::Float32(()), Value::Float32(f))) => ("f32".to_string(), format!("{}f32", f.to_string())),
                Some((Type::Float64(()), Value::Float64(f))) => ("f64".to_string(), format!("{}f64", f.to_string())),

                Some((Type::Text(()), Value::Text(_t))) => { fail!() }
                Some((Type::Data(()), Value::Data(_d))) => { fail!() }
                Some((Type::List(_t), Value::List(_p))) => { fail!() }
                Some((Type::Struct(_t), Value::Struct(_p))) => { fail!() }
                Some((Type::Interface(_t), Value::Interface(()))) => { fail!() }
                Some((Type::AnyPointer(()), Value::AnyPointer(_pr))) => { fail!() }
                None => { fail!("unrecognized type") }
                _ => { fail!("type does not match value") }
            };

            output.push(
                Line(format!("pub static {} : {} = {};", styled_name, typ, txt)));
        }

        Some(Node::Annotation( annotation_reader )) => {
            println!("  annotation node:");
            if annotation_reader.get_targets_file() {
                println!("  targets file");
            }
            if annotation_reader.get_targets_const() {
                println!("  targets const");
            }
            // ...
            if annotation_reader.get_targets_annotation() {
                println!("  targets annotation");
            }
        }

        None => ()
    }

    Branch(output)
}



pub fn main() -> std::io::IoResult<()> {
    use std::io::{Writer, File, Truncate, Write};
    use capnp::serialize;
    use capnp::MessageReader;

    let mut inp = std::io::stdin();

    let message = try!(serialize::new_reader(&mut inp, capnp::ReaderOptions::new()));

    let request : schema_capnp::CodeGeneratorRequest::Reader = message.get_root();

    let mut node_map = collections::hashmap::HashMap::<u64, schema_capnp::Node::Reader>::new();
    let mut scope_map = collections::hashmap::HashMap::<u64, Vec<String>>::new();

    let nodes = request.get_nodes();
    for ii in range(0, nodes.size()) {
        node_map.insert(nodes.get(ii).get_id(), nodes.get(ii));
    }

    let files = request.get_requested_files();

    for ii in range(0, files.size()) {
        let requested_file = files.get(ii);
        let id = requested_file.get_id();
        let mut filepath = std::path::Path::new(requested_file.get_filename());


        let imports = requested_file.get_imports();
        for jj in range(0, imports.size()) {
            let import = imports.get(jj);
            let importpath = std::path::Path::new(import.get_name());
            let root_name : String = format!("::{}_capnp",
                                               importpath.filestem_str().unwrap().replace("-", "_"));
            populate_scope_map(&node_map, &mut scope_map, vec!(root_name), import.get_id());
        }

        let root_name : String = format!("{}_capnp",
                                       filepath.filestem_str().unwrap().replace("-", "_"));

        filepath.set_filename(format!("{}.rs", root_name));

        let root_mod = format!("::{}", root_name);

        populate_scope_map(&node_map, &mut scope_map, vec!(root_mod), id);

        let lines = Branch(vec!(Line("#![allow(unused_imports)]".to_string()),
                                Line("#![allow(dead_code)]".to_string()),
                                generate_node(&node_map, &scope_map,
                                              id, root_name.as_slice())));


        let text = stringify(&lines);

        match File::open_mode(&filepath, Truncate, Write) {
            Ok(ref mut writer) => {
                try!(writer.write(text.as_bytes()));
            }
            Err(e) => {fail!("could not open file for writing: {}", e)}
        }
    }
    Ok(())
}
