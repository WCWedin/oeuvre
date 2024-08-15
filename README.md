# Oeuvre

Oeuvre is a minimalist static site generator written in Rust.

The resulting executable accepts a single command line parameter: a path to a folder containing a file named `site.toml` or to a TOML file directly. The configuration options and their default values are as follows:

```
# The input directory, relative to this file's path.
dir = "./"
# The output directory, relative to the input directory.
output_dir = "output/"
# Glob patterns for files in the input directory that should be ignored.
exclude = []
# Glob patterns for template files.
templates = ["templates/**/*.html"]
# Glob patterns for snipppet files.
snippets = ["snippets/**/*.html"]
# Glob patterns for page files.
pages = ["**/*.html"]
# Glob patterns for static content files.
content = ["content/**/*"]
```

The output is a single HTML file for each page, plus all of the static content, using the directory structure of the input directory.

## The Future

Oeuvre is intended to grow and evolve as needed for my own usage, but if you use Oeuvre yourself, I'd love to hear more about your use case.

## The Name

This project was originally going to be named "printer" to reflect its simple logic, but if you type "printer" starting one key left of home position, you get "oeubrwe" instead. This project is named in honor of that very cool typo.
