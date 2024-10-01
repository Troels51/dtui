use chumsky::prelude::*;
use std::{collections::HashMap, u32};
use zbus::zvariant::{self, parsed, ObjectPath, Signature, StructureBuilder};

/// Create a parser from a parsed::Signature.
/// The language that this parses is a human readable version of the dbus format.
/// Arrays are delimited by [], with values seperated by ","
/// Structure are delimited by (), with values seperated by ","
/// Dictionaries are delimited by {}, and with keys and values seperated by ":" and pairs seperated by ","
///
/// # Examples
/// ["first", "second"] is a array with 2 string elements with a signature of "as"
/// {"first": 1, "second": 2} is a dictionary with string as key type and key type of some number, it's signature is "a{su}"
///
/// ```
/// let signature = parsed::Signature::from_str("as").unwrap();
/// let result = get_parser(signature).parse("[\"first\", \"second\"]");
/// assert_eq!(result, Ok(zvariant::Value::Array(vec!["first", "second"].into())));
/// ```
pub fn get_parser(
    signature: parsed::Signature,
) -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    match signature {
        zvariant::parsed::Signature::Unit => todo!(),
        zvariant::parsed::Signature::U8 => parser_u8().boxed(),
        zvariant::parsed::Signature::Bool => parser_bool().boxed(),
        zvariant::parsed::Signature::I16 => parser_i16().boxed(),
        zvariant::parsed::Signature::U16 => parser_u16().boxed(),
        zvariant::parsed::Signature::I32 => parser_i32().boxed(),
        zvariant::parsed::Signature::U32 => parser_u32().boxed(),
        zvariant::parsed::Signature::I64 => parser_i64().boxed(),
        zvariant::parsed::Signature::U64 => parser_u64().boxed(),
        zvariant::parsed::Signature::F64 => parser_f64().boxed(),
        zvariant::parsed::Signature::Str => parser_string().boxed(),
        zvariant::parsed::Signature::Signature => parser_signature().boxed(),
        zvariant::parsed::Signature::ObjectPath => parser_object_path().boxed(),
        zvariant::parsed::Signature::Variant => parser_variant().boxed(),
        zvariant::parsed::Signature::Fd => parser_fd().boxed(),
        zvariant::parsed::Signature::Array(array) => {
            parser_array(array.signature().clone()).boxed()
        }
        zvariant::parsed::Signature::Dict { key, value } => {
            parser_dict(key.signature().clone(), value.signature().clone()).boxed()
        }
        zvariant::parsed::Signature::Structure(structure) => parser_struct(structure).boxed(),
    }
}

fn parser_variant<'a>() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    parser_signature()
        .boxed()
        .then_ignore(just("->"))
        .then_with(|s| match s {
            zvariant::Value::Signature(signature) => get_parser(signature.into())
                .map(|variant| zvariant::Value::Value(Box::new(variant))),
            _ => unreachable!(),
        })
}
//
fn parser_struct<'a>(
    structure: parsed::FieldsSignatures,
) -> impl Parser<char, zvariant::Value<'a>, Error = Simple<char>> {
    let mut element_parsers = structure
        .iter()
        .map(|signature: &zbus::zvariant::parsed::Signature| get_parser(signature.clone()));
    let mut full_parser = just('(').map(|_| Vec::<zvariant::Value<'_>>::new()).boxed(); // The map is there to get types to match as the chain in the loop needs the parser to output a Vec
    full_parser = full_parser.chain(element_parsers.next().unwrap()).boxed(); // The first doesnt get a ',' the rest do
    for element_parser in element_parsers {
        full_parser = full_parser
            .then_ignore(just(",").padded())
            .chain(element_parser)
            .boxed();
    }
    full_parser.map(|fields| {
        let mut builder = StructureBuilder::new();
        for field in fields {
            builder.push_value(field);
        }
        zvariant::Value::Structure(builder.build().unwrap())
    })
}
fn parser_dict<'a>(
    key_type: parsed::Signature,
    value_type: parsed::Signature,
) -> impl Parser<char, zvariant::Value<'a>, Error = Simple<char>> {
    let key_parser = get_parser(key_type.clone());
    let value_parser = get_parser(value_type.clone());
    let member_parser = key_parser
        .then_ignore(just(":").padded())
        .then(value_parser)
        .boxed();
    member_parser
        .clone()
        .chain(just(',').ignore_then(member_parser).repeated())
        .or_not()
        .flatten()
        .delimited_by(just('{'), just('}'))
        .collect::<HashMap<zvariant::Value<'_>, zvariant::Value<'_>>>()
        .map(
            move |m: HashMap<zvariant::Value<'_>, zvariant::Value<'_>>| {
                let mut dict =
                    zvariant::Dict::new(key_type.clone().into(), value_type.clone().into());
                for (k, v) in m {
                    dict.append(k, v).expect("Could not append to key value pair, this should not happen if types are correct");
                }
                zvariant::Value::Dict(dict)
            },
        )
}

fn parser_array<'a>(
    signature: parsed::Signature,
) -> impl Parser<char, zvariant::Value<'a>, Error = Simple<char>> {
    let element_parser = get_parser(signature.clone()).boxed();
    element_parser
        .clone()
        .chain(just(',').ignore_then(element_parser.clone()).repeated())
        .or_not()
        .flatten()
        .delimited_by(just('['), just(']'))
        .map(move |v: Vec<zvariant::Value<'_>>| {
            let mut array: zvariant::Array<'_> = zvariant::Array::new(signature.clone().into());
            for element in v {
                array
                    .append(element)
                    .expect("The type was somehow incorrect in an inner array");
            }
            zvariant::Value::Array(array)
        })
}
// TODO: Can these be made generic, not sure how as they are generic over the Value type which is enums
// TODO: Validation on sizes of numbers
fn parser_u8() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    text::digits(10)
        .labelled("u8")
        .map(|s: String| zvariant::Value::U8(s.parse().unwrap()))
        .padded()
}
fn parser_u16() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    text::digits(10)
        .labelled("u16")
        .map(|s: String| zvariant::Value::U16(s.parse().unwrap()))
        .padded()
}
fn parser_i16() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    just('-')
        .or_not()
        .chain::<char, _, _>(text::digits(10))
        .collect::<String>()
        .map(|s: String| zvariant::Value::I16(s.parse().unwrap()))
        .labelled("i16")
        .padded()
}
fn parser_u32() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    text::digits(10)
        .labelled("u32")
        .map(|s: String| zvariant::Value::U32(s.parse().unwrap()))
        .padded()
}
fn parser_i32() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    just('-')
        .or_not()
        .chain::<char, _, _>(text::digits(10))
        .collect::<String>()
        .map(|s: String| zvariant::Value::I32(s.parse().unwrap()))
        .padded()
}
fn parser_u64() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    text::digits(10)
        .labelled("u64")
        .map(|s: String| zvariant::Value::U64(s.parse().unwrap()))
        .padded()
}
fn parser_i64() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    just('-')
        .or_not()
        .chain::<char, _, _>(text::digits(10))
        .collect::<String>()
        .labelled("i64")
        .map(|s: String| zvariant::Value::I64(s.parse().unwrap()))
        .padded()
}
fn parser_f64() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    just('-')
        .or_not()
        .chain::<char, _, _>(text::digits(10))
        .chain::<char, _, _>(just('.').chain(text::digits(10)).or_not().flatten())
        .collect::<String>()
        .labelled("f64")
        .map(|s: String| zvariant::Value::F64(s.parse().unwrap()))
        .padded()
}
fn parser_bool() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    just("true")
        .map(|_| zvariant::Value::Bool(true))
        .or(just("false").map(|_| zvariant::Value::Bool(false)))
        .labelled("bool")
        .padded()
}
fn parser_string() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    let escape = just('\\').ignore_then(
        just('\\')
            .or(just('/'))
            .or(just('"'))
            .or(just('b').to('\x08'))
            .or(just('f').to('\x0C'))
            .or(just('n').to('\n'))
            .or(just('r').to('\r'))
            .or(just('t').to('\t'))
            .or(just('u').ignore_then(
                filter(|c: &char| c.is_ascii_hexdigit())
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .validate(|digits, span, emit| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emit(Simple::custom(span, "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            )),
    );
    let string = just('"')
        .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map(|s| zvariant::Value::Str(s.into()))
        .labelled("string");
    string
        .recover_with(skip_then_retry_until(['}', ']']))
        .padded()
}
fn parser_signature() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    let escape = just('\\').ignore_then(
        just('\\')
            .or(just('/'))
            .or(just('"'))
            .or(just('b').to('\x08'))
            .or(just('f').to('\x0C'))
            .or(just('n').to('\n'))
            .or(just('r').to('\r'))
            .or(just('t').to('\t'))
            .or(just('u').ignore_then(
                filter(|c: &char| c.is_ascii_hexdigit())
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .validate(|digits, span, emit| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emit(Simple::custom(span, "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            )),
    );
    let string = just('"')
        .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .try_map(|digits, span| {
            if let Ok(signature) = Signature::try_from(digits) {
                Ok(zvariant::Value::Signature(signature))
            } else {
                Err(Simple::custom(
                    span,
                    "Could not parse signature from string value",
                ))
            }
        })
        .labelled("signature");

    string
        .recover_with(skip_then_retry_until(['}', ']']))
        .padded()
}
fn parser_object_path() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    let escape = just('\\').ignore_then(
        just('\\')
            .or(just('/'))
            .or(just('"'))
            .or(just('b').to('\x08'))
            .or(just('f').to('\x0C'))
            .or(just('n').to('\n'))
            .or(just('r').to('\r'))
            .or(just('t').to('\t'))
            .or(just('u').ignore_then(
                filter(|c: &char| c.is_ascii_hexdigit())
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .validate(|digits, span, emit| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emit(Simple::custom(span, "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            )),
    );
    let string = just('"')
        .ignore_then(filter(|c| *c != '\\' && *c != '"').or(escape).repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .try_map(|digits, span| {
            if let Ok(path) = ObjectPath::try_from(digits) {
                Ok(zvariant::Value::ObjectPath(path))
            } else {
                Err(Simple::custom(
                    span,
                    "Could not parse object path from string value",
                ))
            }
        })
        .labelled("object_path");

    string
        .recover_with(skip_then_retry_until(['}', ']']))
        .padded()
}

fn parser_fd() -> impl Parser<char, zvariant::Value<'static>, Error = Simple<char>> {
    empty().try_map(|(), span| Err(Simple::custom(span, "Cannot parse file descriptors")))
}

#[cfg(test)]
fn test_generic_signature(src: &'static str, signature: &'static str, value: zvariant::Value) {
    use std::str::FromStr;

    let signature = parsed::Signature::from_str(signature).unwrap();
    println!("{}", signature);
    let result = get_parser(signature).parse(src.trim());
    assert_eq!(result, Ok(value));
}
#[test]
fn test_numbers() {
    test_generic_signature("5", "y", zvariant::Value::U8(5));
    test_generic_signature("5", "n", zvariant::Value::I16(5));
    test_generic_signature("-5", "n", zvariant::Value::I16(-5));
    test_generic_signature("5", "q", zvariant::Value::U16(5));
    test_generic_signature("5", "i", zvariant::Value::I32(5));
    test_generic_signature("-5", "i", zvariant::Value::I32(-5));
    test_generic_signature("5", "u", zvariant::Value::U32(5));
    test_generic_signature("5", "x", zvariant::Value::I64(5));
    test_generic_signature("-5", "x", zvariant::Value::I64(-5));
    test_generic_signature("5", "t", zvariant::Value::U64(5));
}
#[test]
fn test_float() {
    test_generic_signature("5.0", "d", zvariant::Value::F64(5.0));
}

#[test]
fn test_string() {
    test_generic_signature(r#""asd""#, "s", zvariant::Value::Str("asd".into()));
}

#[test]
fn test_signature() {
    test_generic_signature(
        r#""s""#,
        "g",
        zvariant::Value::Signature(Signature::try_from("s").unwrap()),
    );
    test_generic_signature(
        r#""(ss)""#,
        "g",
        zvariant::Value::Signature(Signature::try_from("(ss)").unwrap()),
    );
    test_generic_signature(
        r#""as""#,
        "g",
        zvariant::Value::Signature(Signature::try_from("as").unwrap()),
    );
    use std::str::FromStr;
    let signature = parsed::Signature::from_str("g").unwrap();
    let result = get_parser(signature).parse("k"); // k is not a valid signature
    assert!(result.is_err());
}

#[test]
fn test_object_path() {
    test_generic_signature(
        r#""/""#,
        "o",
        zvariant::Value::ObjectPath(ObjectPath::try_from("/").unwrap()),
    );
    test_generic_signature(
        r#""/test""#,
        "o",
        zvariant::Value::ObjectPath(ObjectPath::try_from("/test").unwrap()),
    );
    test_generic_signature(
        r#""/test/test2""#,
        "o",
        zvariant::Value::ObjectPath(ObjectPath::try_from("/test/test2").unwrap()),
    );
    use std::str::FromStr;

    let signature = parsed::Signature::from_str("o").unwrap();
    let result = get_parser(signature).parse("k"); // k is not a valid object path
    assert!(result.is_err());
    let signature = parsed::Signature::from_str("o").unwrap();
    let result = get_parser(signature).parse("//"); // // is not a valid object path
    assert!(result.is_err());
}

#[test]
fn test_bool() {
    test_generic_signature("true", "b", zvariant::Value::Bool(true));
    test_generic_signature("false", "b", zvariant::Value::Bool(false));
}

#[test]
fn test_array() {
    test_generic_signature(
        "[1,2,3,4]",
        "ai",
        zvariant::Value::Array(vec![1, 2, 3, 4].into()),
    );
    // One element
    test_generic_signature("[1]", "ai", zvariant::Value::Array(vec![1].into()));
    // array of array
    let expected_signature = Signature::from_static_str("ai").unwrap();
    let mut expected = zvariant::Array::new(expected_signature);
    expected
        .append(zvariant::Value::Array(vec![1].into()))
        .unwrap();
    expected
        .append(zvariant::Value::Array(vec![2].into()))
        .unwrap();
    expected
        .append(zvariant::Value::Array(vec![3].into()))
        .unwrap();
    expected
        .append(zvariant::Value::Array(vec![4].into()))
        .unwrap();
    test_generic_signature("[[1],[2],[3],[4]]", "aai", zvariant::Value::Array(expected));
    // array of strings
    test_generic_signature(
        r#"["a","b","c","d"]"#,
        "as",
        zvariant::Value::Array(vec!["a", "b", "c", "d"].into()),
    );
}

#[test]
fn test_dict() {
    test_generic_signature(
        r#"{"a": "b", "c":"d"}"#,
        "a{ss}",
        zvariant::Value::Dict(HashMap::from([("a", "b"), ("c", "d")]).into()),
    )
}

#[test]
fn test_struct() {
    test_generic_signature(
        r#"("5", 1)"#,
        "(si)",
        zvariant::Value::Structure(zvariant::Structure::from(("5", 1))),
    );
}

#[test]
fn test_variant() {
    test_generic_signature(
        r#""u"->5"#,
        "v",
        zvariant::Value::Value(Box::new(zvariant::Value::U32(5))),
    );
}
