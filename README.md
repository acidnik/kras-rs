kras
====

kras - Detect, highlight and pretty print structured data

This tool can find structured data of any kind inside of plain string, parse it and pretty-print it:

![](https://github.com/acidnik/kras-rs/raw/master/screenshot.png)

It can detect and parse almost any kind of data:  
json  
python  
rust  

and probably many more. Don't hesitate to open an issue if your data wasn't processed correctly

USAGE:
======
```
    kras [FLAGS] [OPTIONS] [input]...

FLAGS:
        --debug          debug mode
    -C, --force-color    alias for --color yes
    -h, --help           Prints help information
    -s, --sort           sort keys
    -r, --recursive      try to parse nested strings
    -V, --version        Prints version information

OPTIONS:
    -c, --color <color>      colorize output [default: auto]  [possible values: yes, no, auto]
    -i, --indent <indent>    indentation. 0 to disable (colorization is stil performed) [default: 2]
    -w, --width <width>      maximum width of output [default: 80]

ARGS:
    <input>...    Input files or stdin
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
