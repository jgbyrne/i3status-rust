# Copy to appropriate directory, like ~/.config/sway/
# In ~/.config/sway/config - pipe i3status-rs into spacer.py:
#     status_command i3status-rs ..... | python ..../spacer.py maxwidth
# Where maxwidth is the maximum desired width of the bar in characters

import sys

width = int(sys.argv[1])

for line in sys.stdin:
        if "<<>>" in line:
            parts = line.split('"full_text"')[1:]
            acc = 0
            for p in parts:
                if "<<>>" in p:
                    continue
                s = False
                for c in p:
                    if c == '"':
                        if s:
                            break
                        s = True
                    elif s:
                        acc += 1
            pad = width - acc
            if pad <= 0:
                print(line.replace("<<>>", "|"))
            else:
                num = len(parts) - 1
                print(line.replace("<<>>", "|" * (pad // num)))
        else:
            print(line)
