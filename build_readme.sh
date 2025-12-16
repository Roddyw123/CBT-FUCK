#!/bin/bash
# Build README.md from modular docs files

# Concatenate all markdown files in docs/ to README.md
# Files are concatenated in alphabetical order
cat docs/*.md > README.md

echo "README.md generated successfully from docs/"
