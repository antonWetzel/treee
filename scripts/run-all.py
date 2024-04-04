import os
import subprocess
import random

SOURCE = "D:/data/"
OUTPUT = "D:/projects/treee/cwd/auto/"

TYPES = [
    # "ALS",
    "ULS",
    # "TLS",
]

FOLDERS = [
    "BR01",
    # "KA09",
    # "KA10",
    # "KA11",
    # "SP02",
    # "",
]

subprocess.run(["cargo", "install", "--path=treee"])

for (directory, folders, files) in os.walk(SOURCE):
    if "single_trees" in directory:
        continue
    random.shuffle(files)
    for file in files:
        if not file.endswith(".laz"):
            continue
        if all(map(lambda t: t not in file, TYPES)):
            continue
        if all(map(lambda t: t not in file, FOLDERS)):
            continue
        path = directory + "/" + file
        print(path)
        subprocess.run(["treee", "importer", path, "-o=" + OUTPUT + file[:-4]])
        break