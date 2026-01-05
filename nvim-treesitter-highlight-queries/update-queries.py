import io
import pathlib
import re
import urllib.request
import zipfile

hash = input("Commit hash of nvim-treesitter to use: ")

with open("nvim-treesitter-commit-hash.txt", "w") as f:
    f.write(hash)

with urllib.request.urlopen(f"https://github.com/nvim-treesitter/nvim-treesitter/archive/{hash}.zip") as f:
    data = f.read()

zip = zipfile.ZipFile(io.BytesIO(data))

name_regex = re.compile(f"nvim-treesitter-{hash}/runtime/queries/([^/]+)/highlights.scm")

queries = pathlib.Path("queries")
queries.mkdir(exist_ok=True)

for name in zip.namelist():
    match = name_regex.match(name)
    if match:
        id = match.groups()[0]
        # Flat structure is a bit nicer to work with
        file = queries / f"{id}-highlights.scm"
        file.write_bytes(zip.read(name))
