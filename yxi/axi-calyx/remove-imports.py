import sys

readfile = open(sys.argv[1], 'r')
writefile = open(sys.argv[2], 'w')

for line in readfile.readlines():
   
    # reading all lines that begin 
    # with "TextGenerator"
    if "import" not in line:
        writefile.write(line)
        
readfile.close()
writefile.close()
        