import sys

width = int(sys.argv[1])

for line in sys.stdin:
        if "<<>>" in line:
            parts = line.split('"full_text"')[1:]
            acc = 0
            for p in parts:
                s = False
                for c in parts:
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
                print(num)
                print(line.replace("<<>>", "|" * (pad // num)))
        else:
            print(line)
