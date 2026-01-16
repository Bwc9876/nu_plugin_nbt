use std::io::Cursor;

use nu_plugin::PluginCommand;
use nu_protocol::{
    Category, IntoPipelineData, IntoValue, LabeledError, PipelineData, Record, Signature, Span,
    Type, Value, record,
};
use simdnbt::{
    FromNbtTag,
    borrow::{Nbt, NbtCompound, NbtList, NbtTag},
};

use crate::tags;

pub struct FromNbt;

fn tag_val(tag: u8, list_type: Option<u8>, val: Value, span: Span) -> Value {
    Value::record(
        record!("nbt_tag" => Value::int(tag as i64, span), "nbt_list_type" => list_type.map(|t| t as i64).into_value(span), "value" => val),
        span,
    )
}

macro_rules! arm {
    ($struct:ident, $func:ident, $tag:ident, $span:ident) => {
        Ok($struct::$func(&$tag).unwrap().into_value($span))
    };
}

macro_rules! arm_array {
    ($struct:ident, $func:ident, $tag:ident, $span:ident) => {
        Ok(Vec::from($struct::$func(&$tag).unwrap()).into_value($span))
    };
}

macro_rules! match_tag {
    ($id:ident, $tag:ident, $struct:ident, $span:ident, [$($tag_id:pat => $ty:ident$(,)?)*], [$($arrtag_id:pat => $func:ident$(,)?)*], [$($rest_id:pat => $rest:expr$(,)?)*], $def:expr) => {
        match $id {
            $($tag_id => arm!($struct, $ty, $tag, $span),)*
            $($arrtag_id => arm_array!($struct, $func, $tag, $span),)*
            $($rest_id => $rest,)*
            _ => $def,
        }
    }
}

fn nbt_to_val<'a: 'tape, 'tape>(
    tag: NbtTag<'a, 'tape>,
    span: Span,
    do_tags: bool,
) -> Result<Value, LabeledError> {
    // Roundabout way of doing this, unwraps are safe bc we're checking with [NbtTag::id] first and
    // the conversion functions do that too
    let id = tag.id();
    let list_type = tag.list().map(|l| l.id());
    let res = match_tag!(id, tag, NbtTag, span, [
        tags::BYTE_ID => byte,
        tags::SHORT_ID => short,
        tags::INT_ID => int,
        tags::LONG_ID => long,
        tags::FLOAT_ID => float,
        tags::DOUBLE_ID => double,
    ], [
        tags::BYTE_ARRAY_ID => byte_array,
        tags::INT_ARRAY_ID => int_array,
        tags::LONG_ARRAY_ID => long_array,
    ], [
        tags::STRING_ID => Ok(String::from_nbt_tag(tag).unwrap().into_value(span)),
        tags::COMPOUND_ID => compound_to_record(tag.compound().unwrap(), span, do_tags).map(|r| r.into_value(span)),
        tags::LIST_ID => list_to_val(tag.list().unwrap(), span, do_tags),
    ], Err(LabeledError::new(format!("Unknown NBT tag {id}"))));

    if do_tags {
        res.map(|v| tag_val(id, list_type, v, span))
    } else {
        res
    }
}

fn list_to_val<'a: 'tape, 'tape>(
    list: NbtList<'a, 'tape>,
    span: Span,
    do_tags: bool,
) -> Result<Value, LabeledError> {
    let id = list.id();
    match_tag!(id, list, NbtList, span, [
        tags::SHORT_ID => shorts,
        tags::INT_ID => ints,
        tags::LONG_ID => longs,
        tags::FLOAT_ID => floats,
        tags::DOUBLE_ID => doubles,
    ], [
        tags::BYTE_ID => bytes,
    ], [
        tags::STRING_ID => Ok(list.strings().unwrap().iter().map(|s| s.to_string()).collect::<Vec<_>>().into_value(span)),
        tags::BYTE_ARRAY_ID => Ok(list.byte_arrays().unwrap().iter().map(|a| a.iter().copied().collect()).collect::<Vec<Vec<_>>>().into_value(span)),
        tags::INT_ARRAY_ID => Ok(list.int_arrays().unwrap().iter().map(|a| a.to_vec()).collect::<Vec<_>>().into_value(span)),
        tags::LONG_ARRAY_ID => Ok(list.long_arrays().unwrap().iter().map(|a| a.to_vec()).collect::<Vec<_>>().into_value(span)),
        tags::LIST_ID => Ok(list.lists().unwrap().into_iter().map(|l| {
            let inner_id = l.id();
            let val = list_to_val(l, span, do_tags);
            if do_tags { val.map(|v| tag_val(tags::LIST_ID, Some(inner_id), v, span)) } else { val }
        }).collect::<Result<Vec<_>, _>>()?.into_value(span)),
        tags::COMPOUND_ID => Ok(list.compounds().unwrap().into_iter().map(|c| {
            let record = compound_to_record(c, span, do_tags);
            record.map(|r| r.into_value(span))
        }).collect::<Result<Vec<_>, _>>()?.into_value(span)),
        tags::END_ID => Ok(Vec::<u8>::new().into_value(span)),
    ], Err(LabeledError::new(format!("Unknown NBT list type: {id}"))))
}

fn compound_to_record<'a: 'tape, 'tape>(
    compound: NbtCompound<'a, 'tape>,
    span: Span,
    do_tags: bool,
) -> Result<Record, LabeledError> {
    compound
        .iter()
        .map(|(s, v)| {
            let key = s.to_string();
            let value = nbt_to_val(v, span, do_tags);
            value.map(|value| (key, value))
        })
        .collect()
}

fn parse_nbt(
    src: &[u8],
    src_span: Span,
    call_span: Span,
    do_tags: bool,
) -> Result<PipelineData, LabeledError> {
    let mut decoded_src_decoder = flate2::read::GzDecoder::new(&src[..]);
    let mut input = Vec::new();
    if std::io::Read::read_to_end(&mut decoded_src_decoder, &mut input).is_err() {
        // oh probably wasn't gzipped then
        input = src.to_vec();
    }
    let input = input.as_slice();

    let nbt = simdnbt::borrow::read(&mut Cursor::new(input)).map_err(|err| {
        let msg = format!("Failed to parse NBT data:\n{err:?}");
        LabeledError::new(&msg).with_label("Invalid NBT data passed in".to_string(), src_span)
    })?;

    match nbt {
        Nbt::Some(nbt) => {
            let record = compound_to_record(nbt.as_compound(), call_span, do_tags)?;
            Ok(Value::record(record, call_span))
        }
        Nbt::None => Ok(Value::nothing(call_span)),
    }
    .map(Value::into_pipeline_data)
}

impl PluginCommand for FromNbt {
    type Plugin = crate::NbtPlugin;

    fn name(&self) -> &str {
        "from nbt"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .input_output_type(Type::Binary, Type::record())
            .switch("with-tags", "Whether to output NBT tag info (use this if you want a format that you can save data from)", Some('t'))
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert from a stream of raw NBT data"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        let do_tags = call.has_flag("with-tags")?;
        match input {
            PipelineData::Value(v, _) => {
                let data = v.as_binary()?;
                parse_nbt(&data, v.span(), call.head, do_tags)
            }
            PipelineData::ByteStream(stream, _) => {
                let span = stream.span();
                let data = stream.into_bytes()?;
                parse_nbt(&data, span, call.head, do_tags)
            }
            _ => Err(
                LabeledError::new("Expected value or byte stream from pipeline").with_label(
                    format!("requires value or byte stream; got {}", input.get_type(),),
                    call.head,
                ),
            ),
        }
    }
}
