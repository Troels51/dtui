use chumsky::input::InputRef;
use chumsky::prelude::*;
use std::{collections::HashMap, str::FromStr, u32};
use zbus::zvariant::{self, ObjectPath, Signature, StructureBuilder};

/// The extra parser state used by every parser in this module.
type Extra<'src> = extra::Err<Rich<'src, char>>;
/// A boxed parser over a string input, producing a dbus value.
type ValueParser<'src> = Boxed<'src, 'src, &'src str, zvariant::Value<'static>, Extra<'src>>;

/// Create a parser from a Signature.
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
/// let signature = Signature::from_str("as").unwrap();
/// let result = get_parser(signature).parse("[\"first\", \"second\"]").into_result();
/// assert_eq!(result, Ok(zvariant::Value::Array(vec!["first", "second"].into())));
/// ```
pub fn get_parser<'src>(signature: Signature) -> ValueParser<'src> {
    match signature {
        zvariant::Signature::Unit => todo!(),
        zvariant::Signature::U8 => parser_u8().boxed(),
        zvariant::Signature::Bool => parser_bool().boxed(),
        zvariant::Signature::I16 => parser_i16().boxed(),
        zvariant::Signature::U16 => parser_u16().boxed(),
        zvariant::Signature::I32 => parser_i32().boxed(),
        zvariant::Signature::U32 => parser_u32().boxed(),
        zvariant::Signature::I64 => parser_i64().boxed(),
        zvariant::Signature::U64 => parser_u64().boxed(),
        zvariant::Signature::F64 => parser_f64().boxed(),
        zvariant::Signature::Str => parser_string().boxed(),
        zvariant::Signature::Signature => parser_signature().boxed(),
        zvariant::Signature::ObjectPath => parser_object_path().boxed(),
        zvariant::Signature::Variant => parser_variant(),
        zvariant::Signature::Fd => parser_fd().boxed(),
        zvariant::Signature::Array(child) => parser_array(child.signature().clone()).boxed(),
        zvariant::Signature::Dict { key, value } => {
            parser_dict(key.signature().clone(), value.signature().clone()).boxed()
        }
        zvariant::Signature::Structure(fields) => parser_struct(fields).boxed(),
    }
}

/// The inner parser of a variant depends on the signature that precedes it, so it can only be
/// built once that signature has been parsed. `custom` gives us access to the input so we can
/// run a parser that is constructed on the fly.
fn parser_variant<'src>() -> ValueParser<'src> {
    custom::<_, &'src str, _, Extra<'src>>(|input: &mut InputRef<'src, '_, _, _>| {
        let signature = match input.parse(parser_signature())? {
            zvariant::Value::Signature(signature) => signature,
            _ => unreachable!(),
        };
        input.parse(just("->"))?;
        let value = input.parse(get_parser(signature))?;
        Ok(zvariant::Value::Value(Box::new(value)))
    })
    .boxed()
}
//
fn parser_struct<'src>(
    structure: zvariant::signature::Fields,
) -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    let mut element_parsers = structure
        .iter()
        .map(|signature: &zbus::zvariant::Signature| get_parser(signature.clone()));
    // The first element doesn't get a ',' the rest do
    let mut full_parser = element_parsers
        .next()
        .unwrap()
        .map(|value| vec![value])
        .boxed();
    for element_parser in element_parsers {
        full_parser = full_parser
            .then_ignore(just(",").padded())
            .then(element_parser)
            .map(|(mut fields, field)| {
                fields.push(field);
                fields
            })
            .boxed();
    }
    let full_parser = full_parser.delimited_by(just('(').padded(), just(')').padded());

    full_parser.map(|fields| {
        let mut builder = StructureBuilder::new();
        for field in fields {
            builder.push_value(field);
        }
        zvariant::Value::Structure(builder.build().unwrap())
    })
}
fn parser_dict<'src>(
    key_type: Signature,
    value_type: Signature,
) -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    let key_parser = get_parser(key_type.clone());
    let value_parser = get_parser(value_type.clone());
    let member_parser = key_parser
        .then_ignore(just(":").padded())
        .then(value_parser)
        .boxed();
    member_parser
        .separated_by(just(',').padded())
        .collect::<HashMap<zvariant::Value<'static>, zvariant::Value<'static>>>()
        .delimited_by(just('{').padded(), just('}').padded())
        .map(
            move |m: HashMap<zvariant::Value<'_>, zvariant::Value<'_>>| {
                let mut dict =
                    zvariant::Dict::new(&key_type, &value_type);
                for (k, v) in m {
                    dict.append(k, v).expect("Could not append to key value pair, this should not happen if types are correct");
                }
                zvariant::Value::Dict(dict)
            },
        )
}

fn parser_array<'src>(
    signature: Signature,
) -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    let element_parser = get_parser(signature.clone());
    element_parser
        .separated_by(just(',').padded())
        .collect::<Vec<zvariant::Value<'static>>>()
        .delimited_by(just('['), just(']'))
        .map(move |v: Vec<zvariant::Value<'_>>| {
            let mut array: zvariant::Array<'_> = zvariant::Array::new(&signature);
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
fn parser_u8<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    text::digits(10)
        .to_slice()
        .labelled("u8")
        .map(|s: &str| zvariant::Value::U8(s.parse().unwrap()))
        .padded()
}
fn parser_u16<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    text::digits(10)
        .to_slice()
        .labelled("u16")
        .map(|s: &str| zvariant::Value::U16(s.parse().unwrap()))
        .padded()
}
fn parser_i16<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    just('-')
        .or_not()
        .then(text::digits(10))
        .to_slice()
        .map(|s: &str| zvariant::Value::I16(s.parse().unwrap()))
        .labelled("i16")
        .padded()
}
fn parser_u32<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    text::digits(10)
        .to_slice()
        .labelled("u32")
        .map(|s: &str| zvariant::Value::U32(s.parse().unwrap()))
        .padded()
}
fn parser_i32<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    just('-')
        .or_not()
        .then(text::digits(10))
        .to_slice()
        .map(|s: &str| zvariant::Value::I32(s.parse().unwrap()))
        .padded()
}
fn parser_u64<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    text::digits(10)
        .to_slice()
        .labelled("u64")
        .map(|s: &str| zvariant::Value::U64(s.parse().unwrap()))
        .padded()
}
fn parser_i64<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    just('-')
        .or_not()
        .then(text::digits(10))
        .to_slice()
        .labelled("i64")
        .map(|s: &str| zvariant::Value::I64(s.parse().unwrap()))
        .padded()
}
fn parser_f64<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    just('-')
        .or_not()
        .then(text::digits(10))
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice()
        .labelled("f64")
        .map(|s: &str| zvariant::Value::F64(s.parse().unwrap()))
        .padded()
}
fn parser_bool<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone
{
    just("true")
        .map(|_| zvariant::Value::Bool(true))
        .or(just("false").map(|_| zvariant::Value::Bool(false)))
        .labelled("bool")
        .padded()
}

/// The quoted string literal that is shared by the string, signature and object path parsers.
fn quoted_string<'src>() -> impl Parser<'src, &'src str, String, Extra<'src>> + Clone {
    let escape = just('\\').ignore_then(choice((
        just('\\'),
        just('/'),
        just('"'),
        just('b').to('\x08'),
        just('f').to('\x0C'),
        just('n').to('\n'),
        just('r').to('\r'),
        just('t').to('\t'),
        just('u').ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_hexdigit())
                .repeated()
                .exactly(4)
                .to_slice()
                .validate(|digits: &str, extra, emitter| {
                    char::from_u32(u32::from_str_radix(digits, 16).unwrap()).unwrap_or_else(|| {
                        emitter.emit(Rich::custom(extra.span(), "invalid unicode character"));
                        '\u{FFFD}' // unicode replacement character
                    })
                }),
        ),
    )));
    just('"')
        .ignore_then(none_of("\\\"").or(escape).repeated().collect::<String>())
        .then_ignore(just('"'))
}

fn parser_string<'src>()
-> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    quoted_string()
        .map(|s| zvariant::Value::Str(s.into()))
        .labelled("string")
        .recover_with(skip_then_retry_until(any().ignored(), one_of("}]").ignored()))
        .padded()
}
fn parser_signature<'src>()
-> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    quoted_string()
        .try_map(|digits: String, span| {
            if let Ok(signature) = Signature::from_str(digits.as_str()) {
                Ok(zvariant::Value::Signature(signature))
            } else {
                Err(Rich::custom(
                    span,
                    "Could not parse signature from string value",
                ))
            }
        })
        .labelled("signature")
        .recover_with(skip_then_retry_until(any().ignored(), one_of("}]").ignored()))
        .padded()
}
fn parser_object_path<'src>()
-> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    quoted_string()
        .try_map(|digits: String, span| {
            if let Ok(path) = ObjectPath::try_from(digits) {
                Ok(zvariant::Value::ObjectPath(path))
            } else {
                Err(Rich::custom(
                    span,
                    "Could not parse object path from string value",
                ))
            }
        })
        .labelled("object_path")
        .recover_with(skip_then_retry_until(any().ignored(), one_of("}]").ignored()))
        .padded()
}

fn parser_fd<'src>() -> impl Parser<'src, &'src str, zvariant::Value<'static>, Extra<'src>> + Clone {
    empty().try_map(|(), span| Err(Rich::custom(span, "Cannot parse file descriptors")))
}

#[cfg(test)]
fn test_generic_signature(src: &'static str, signature: &'static str, value: zvariant::Value) {
    use std::str::FromStr;

    let signature = Signature::from_str(signature).unwrap();
    println!("{}", signature);
    let result = get_parser(signature).parse(src.trim()).into_result();
    dbg!(&result);
    dbg!(&value);
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
    let signature = Signature::from_str("g").unwrap();
    let result = get_parser(signature).parse("k").into_result(); // k is not a valid signature
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

    let signature = Signature::from_str("o").unwrap();
    let result = get_parser(signature).parse("k").into_result(); // k is not a valid object path
    assert!(result.is_err());
    let signature = Signature::from_str("o").unwrap();
    let result = get_parser(signature).parse("//").into_result(); // // is not a valid object path
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
    let expected_signature = Signature::from_str("ai").unwrap();
    let mut expected = zvariant::Array::new(&expected_signature);
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
    // Array of structs
    fn one_element_struct(s: String) -> zvariant::Value<'static> {
        zvariant::Value::Structure(
            zvariant::StructureBuilder::new()
                .add_field(Into::<zvariant::Str>::into(s))
                .build()
                .unwrap(),
        )
    }
    let struct_signature = Signature::Structure(zvariant::signature::Fields::Static {
        fields: &[&Signature::Str],
    });
    let mut array = zvariant::Array::new(&struct_signature);
    let _ = array.append(one_element_struct("a".to_string()));
    let _ = array.append(one_element_struct("b".to_string()));
    let _ = array.append(one_element_struct("c".to_string()));
    let _ = array.append(one_element_struct("d".to_string()));
    test_generic_signature(r#"[("a"),("b"),("c"),("d")]"#, "a(s)", array.into());

    // Array of dicts

    let mut array = zvariant::Array::new(&Signature::Dict {
        key: Signature::Str.into(),
        value: Signature::Str.into(),
    });
    let _ = array.append(zvariant::Value::Dict(
        HashMap::from([("a", "1"), ("b", "2")]).into(),
    ));
    let _ = array.append(zvariant::Value::Dict(
        HashMap::from([("c", "3"), ("d", "4")]).into(),
    ));
    test_generic_signature(
        r#"[{"a": "1", "b":"2"}, {"c": "3", "d":"4"}]"#,
        "aa{ss}",
        array.into(),
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
    test_generic_signature(
        r#"("5", 1, 2, 3, 4, 5)"#,
        "(siiiii)",
        zvariant::Value::Structure(zvariant::Structure::from(("5", 1, 2, 3, 4, 5))),
    );
    // One element structs
    let one_elemenent_structure = zvariant::StructureBuilder::new()
        .add_field(Into::<zvariant::Str>::into("a"))
        .build()
        .unwrap();
    test_generic_signature(
        r#"("a")"#,
        "(s)",
        zvariant::Value::Structure(one_elemenent_structure),
    );
    // Struct with array
    let one_elemenent_structure = zvariant::StructureBuilder::new()
        .add_field(Into::<zvariant::Array>::into(vec!["a"]))
        .build()
        .unwrap();
    test_generic_signature(
        r#"(["a"])"#,
        "(as)",
        zvariant::Value::Structure(zvariant::Structure::from(one_elemenent_structure)),
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

// Test from examples in help view
#[test]
fn test_dict_from_examples() {
    test_generic_signature(
        r#"{"count": 5, "max": 10}"#,
        "a{si}",
        zvariant::Value::Dict(HashMap::from([("count", 5i32), ("max", 10i32)]).into()),
    )
}

#[test]
fn test_struct_from_examples() {
    test_generic_signature(
        r#"("user_name", "john", 42)"#,
        "(ssu)",
        zvariant::Value::Structure(zvariant::Structure::from(("user_name", "john", 42u32))),
    )
}

#[test]
fn test_array_of_struct_from_examples() {
    let struct_signature = Signature::Structure(zvariant::signature::Fields::Static {
        fields: &[&Signature::I32, &Signature::Str],
    });
    let mut array = zvariant::Array::new(&struct_signature);
    let _ = array.append(zvariant::Value::Structure(zvariant::Structure::from((
        1, "a",
    ))));
    let _ = array.append(zvariant::Value::Structure(zvariant::Structure::from((
        2, "b",
    ))));

    test_generic_signature(r#"[(1,"a"),(2,"b")]"#, "a(is)", array.into())
}
