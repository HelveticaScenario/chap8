import sys

def decode(inst):
    nibbles = (inst[0] >> 4, inst[0] & 0xf, inst[1] >> 4, inst[1] & 0xf)

    op = nibbles[0]
    args = nibbles[1:]

    if op == 0x0:
        if args[2] == 0xe:
            return "ret"
    if op == 0x1:
        return "jmp_addr({:x}{:x}{:x})".format(*args)
    if op == 0x2:
        return "call_addr({:x}{:x}{:x})".format(*args)
    if op == 0x3:
        return "se_vx_byte({:x}, {:x}{:x})".format(*args)
    if op == 0x4:
        return "sne_vx_byte({:x}, {:x}{:x})".format(*args)
    if op == 0x6:
        return "ld_vx_byte({:x}, {:x}{:x})".format(*args)
    if op == 0x7:
        return "add_vx_byte({:x}, {:x}{:x})".format(*args)
    if op == 0x8:
        if args[2] == 0x0:
            return "ld_vx_vy({:x}, {:x})".format(*args[1:3])
        if args[2] == 0x2:
            return "and_vx_vy({:x}, {:x})".format(*args[1:3])
        if args[2] == 0x5:
            return "sub_vx_vy({:x}, {:x})".format(*args[1:3])
    if op == 0xa:
        return "ld_i_addr({:x}{:x}{:x})".format(*args)
    if op == 0xc:
        return "rnd_vx_byte({:x}, {:x}{:x})".format(*args)
    if op == 0xd:
        return "drw_vx_vy_nibble({:x}, {:x}, {:x})".format(*args)
    if op == 0xf:
        if args[2] == 0xe:
            return "add_i_vx({:x})".format(args[1])
    else:
        return "INVALID"

def main():
    file_name = sys.argv[1]
    with open(file_name, "rb") as fi:
        inst = fi.read(2)

        # assume instructions start at 0x200
        addr = 0x200
        while inst != b"":
            print("{:03x} {:02x}{:02x}: {}".format(addr, inst[0], inst[1], decode(inst)))
            inst = fi.read(2)
            addr += 2

if __name__ == "__main__":
    main()
