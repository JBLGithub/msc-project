import os
import re
import subprocess
import sys

command = "tcpdump -i enp3s0 'ip6[24:2] = 0xff02 && ip6[30:2] = 0x5d73 && ip6[36:2] != 0x5d73 && ip6[88:1] != 0x03' -x -l"
#command = "tcpdump -i enp3s0 'ip6[24:2] = 0xff02 && ip6[30:2] = 0x5d73 && ip6[88:1] != 0x03' -x -l"

try:
    os.system('clear')
    process = subprocess.Popen(
        command,
        shell=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    first = True
    for row in iter(process.stdout.readline, b''):
        line = row.rstrip()

        if "IP6" in line:
            if not first:
                print()
            else:
                first = False
            print("******************")

            split_line = line.split(" ")
            print(split_line[2] + " " + split_line[3])
            print(split_line[4])

        if not "0x0000:" in line and not "0x0010:" in line and not "0x0020:" in line and not "IP6" in line:
            
            cleaned_line = re.sub(r'\s*0x[0-9a-fA-F]{4}:\s*', '', line)
            split_line = cleaned_line.split(" ")

            counter = 0
            for segment in split_line:
                print(segment, end=' ')
                counter += 1
                if counter == 4:
                    print()
                    counter = 0

            sys.stdout.flush()
            
except KeyboardInterrupt:
    print()
    print("\nStopping tcpdump...")
finally:
    process.terminate()