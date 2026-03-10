
# nu_plugin_nbt

Read Minecraft nbt in Nushell.

## Installation

### Shell

Build / download the plugin and run `plugin add`.

```nu
plugin add path/to/nu_plugin_nbt
```

Then to use it in your shell, run `plugin use`.

```nu
plugin use nu_plugin_nbt
```

### Script

Build / download the plugin binary. When invoking your script pass its path with `--plugin`.

```nu
nu --plugin path/to/nu_plugin_nbt my_script.nu
```

## Usage

If your file ends in .nbt, nushell will pick nu_plugin_nbt to open it.

```nu
open data.nbt
```

Otherwise, open it raw and pipe it to `from nt`.

```nu
open --raw data.dat | from nbt
```

### Preserve Data Types

Passing `--with-tags` to `from nbt` will tag data with NBT types. 

This is useful if you need to convert back into NBT at some point. Note that this plugin does not handle writing (yet). 


