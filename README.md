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
templates = ["templates/**/*.xml"]
# Glob patterns for snipppet files.
snippets = ["snippets/**/*.xml"]
# Glob patterns for dataset files.
datasets = ["data/*.xml"]
# Glob patterns for data row files.
datarows = ["data/*/**/*.xml"]
# Glob patterns for page files.
pages = ["**/*.xml"]
# Glob patterns for static assets files.
assets = ["assets/**/*"]
```

The output is a single HTML file for each page, plus all of the static assets, using the directory structure of the input directory.

## The Future

Oeuvre is intended to grow and evolve as needed for my own usage, but if you use Oeuvre yourself, I'd love to hear more about your use case.

## The Name

This project was originally going to be named "printer" to reflect its simple logic, but if you type "printer" starting one key left of home position, you get "oeubrwe" instead. This project is named in honor of that very cool typo.
