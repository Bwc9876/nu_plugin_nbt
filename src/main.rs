use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, PluginCommand};

mod from;
mod tags;

pub struct NbtPlugin;

impl Plugin for NbtPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(from::FromNbt)]
    }
}

fn main() {
    serve_plugin(&NbtPlugin, MsgPackSerializer)
}
