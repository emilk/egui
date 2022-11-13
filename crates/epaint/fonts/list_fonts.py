#!/usr/bin/env python
from fontTools.ttLib import TTFont
from fontTools.unicode import Unicode
from itertools import chain
import sys

ttf = TTFont(sys.argv[1], 0, verbose=0, allowVID=0,
             ignoreDecompileErrors=True,
             fontNumber=-1)

chars = chain.from_iterable([y + (Unicode[y[0]],)
                             for y in x.cmap.items()] for x in ttf["cmap"].tables)


all_codepoints = {}

for entry in chars:
    codepoint = entry[0]
    short_name = entry[1]
    long_name = entry[2].lower()
    if False:
        print(f'(0x{codepoint:02X}, "{short_name}", "{long_name}"),')
    else:
        name = short_name if long_name == "????" else long_name
        # print(f'(0x{codepoint:02X}, "{name}"),')
        all_codepoints[codepoint] = name

for codepoint in sorted(all_codepoints.keys()):
    name = all_codepoints[codepoint]
    print(f'(0x{codepoint:02X}, \'{chr(codepoint)}\', "{name}"),')

ttf.close()
