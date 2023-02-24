kras
====

kras - Detect, highlight and pretty print structured data

This tool can find structured data of any kind inside of plain string, parse it and pretty-print it:

![](https://github.com/acidnik/kras-rs/raw/master/screenshot.png)

It can detect and parse almost any kind of data:
* json
* python
* rust

and probably many more. Don't hesitate to open an issue if your data wasn't processed correctly

Installation
============
```
cargo install kras-rs
```


Usage
=====
```
Usage: kras [OPTIONS] [INPUT]...

Arguments:
  [INPUT]...  Input files or stdin

Options:
  -i, --indent <INDENT>  identation. 0 to disable (colorization is still performed) [default: 2]
  -c, --color <COLOR>    colorize output [default: auto] [possible values: auto, yes, no]
  -C, --force-color      alias for --color yes
  -s, --sort             sort keys
  -r, --recursive        try to parse nested strings
  -j, --jobs <JOBS>      number of parallel jobs. Default is num_cpus
  -w, --width <WIDTH>    maximum width of output [default: 80]
  -m, --multiline        look for data spannding several lines. This will read wholle input to memory
      --robust           use more robust, but slower method to detect structured data
      --debug            debut mode
  -h, --help             Print help
  -V, --version          Print version

```

Using with pgcli
================
`kras` really shines when used for reading jsons stored in database. For pgcli add to your `.config/pgcli/config`
```
pager = kras -Csw120 | less -iRXF
```
Now your jsons will be pretty-printed! Hint: use `\x`

Acknowledgement
===============
This tool is powered by these amazing libs: [pom](https://lib.rs/crates/pom) for parsing and [pretty](https://lib.rs/crates/pretty) for pretty-printing

Trivia
======
The name *kras* comes from russian root *крас-* - a beginning of words such as *красивый* (pretty), *красный* (red) and *красить* (to paint).
That's what this app does: makes data pretty and paints it red (but not only red)
