Debugging
=========

Debugging is an important part of operating system development. Running Denuos
in QEMU has the advantage that we can use [GDB][] to interactively debug the
operating system while it is running. QEMU must be started with several flags
in order to facilitate this. Thus the `make debug` target is provided.

These are equivalent.
```
make debug
qemu-system-x86_64 -s -S -d int -no-reboot -cdrom bin/denuos.iso
```



Using GDB
---------

GDB provides an arcane interface, but it is relatively easy to begin using.

Connect to QEMU.
```
(gdb) target remote tcp::1234
Remote debugging using tcp::1234
0x0000fff0 in ?? ()
```

Load debugging symbols.
```
(gdb) symbol-file ./bin/kernel.bin
Reading symbols from ./bin/kernel.bin...done.
```

Set the correct x86-64 architecture.
```
(gdb) set arch i386:x86-64
The target architecture is assumed to be i386:x86-64
```

Set your preferred assembly syntax.
```
(gdb) set disassembly-flavor intel
```


With this setup, you're ready to set a break point somewhere and begin
debugging.

```
(gdb) br kmain
Breakpoint 1 at 0x1030d0: file /home/jason/repo/denuos/src/kernel/lib.rs, line 11.
(gdb) cont
Continuing.
Remote 'g' packet reply is too long: 100000000000000000080000000000000120808000000000f
...
```

It turns out GDB doesn't work very well when the CPU changes modes (real,
protected, long) midway through execution, thus causing the above error.
[The easiest][longfix] workaround is to reset GDB slightly.

```
(gdb) disconnect
Ending remote debugging.
(gdb) set arch i386:x86-64:intel
The target architecture is assumed to be i386:x86-64:intel
(gdb) target remote tcp::1234
Remote debugging using tcp::1234
kernel::kmain () at /home/jason/repo/denuos/src/kernel/lib.rs:11
11      pub extern fn kmain() {
```

To save time, the above workaround is implemented as a `longfix` command
defined in the `.gdbinit` described below.

Now we're ready to begin debugging. Let's inspect the code we're about to
execute.
```
(gdb) x/4i $pc
=> 0x1030d0 <kernel::kmain>:    push   rbp
   0x1030d1 <kernel::kmain+1>:  mov    rbp,rsp
   0x1030d4 <kernel::kmain+4>:  sub    rsp,0x70
   0x1030d8 <kernel::kmain+8>:  call   0x102980 <kernel::vga::clear_screen>
```

`x/4i` says to e<b>x</b>amine 4 instructions at the specified memory address.
`$pc` is the location of the next instruction to be executed. To avoid retyping
this all the time, you can use `display /i $pc` to show the next instruction
after every step.

Let's follow execution into the `kernel::vga::clear_screen` function.
```
(gdb) si 4
kernel::vga::clear_screen () at /home/jason/repo/denuos/src/kernel/vga.rs:30
30      pub fn clear_screen() {
```

`si 4` says to <b>s</b>tep <b>i</b>nto 4 instructions. This will follow into
function calls. If you ever want to skip over a function call, use `ni` instead.
If you hit the breakpoint we defined earlier, just use `si` again to continue
one instruction at a time.

If you ever get lost, try taking a look at the stack.
```
(gdb) info stack
#0  kernel::vga::clear_screen () at /home/jason/repo/denuos/src/kernel/vga.rs:30
#1  0x00000000001030dd in kernel::kmain ()
    at /home/jason/repo/denuos/src/kernel/lib.rs:12
#2  0x0000000000100755 in start64 ()
#3  0x0000000000107023 in p2_table ()
#4  0x0000000000000000 in ?? ()
```

If you can't understand why a function is misbehaving, try inspecting
its registers.
```
(gdb) print $rax
$1 = 16
(gdb) info registers
rax            0x10     16
rbx            0x800    2048
rcx            0x80802001       2155880449
rdx            0x78bfbfd        126614525
rsi            0x0      0
rdi            0x0      0
rbp            0x105ff0 0x105ff0
rsp            0x105f78 0x105f78
r8             0x0      0
r9             0x0      0
r10            0x0      0
r11            0x0      0
r12            0x0      0
r13            0x0      0
r14            0x0      0
r15            0x0      0
rip            0x102980 0x102980 <kernel::vga::clear_screen>
eflags         0x200002 [ ID ]
cs             0x8      8
ss             0x10     16
ds             0x10     16
es             0x10     16
fs             0x18     24
gs             0x18     24
```

.gdbinit
--------
Consider placing the following commands in a `.gdbinit` file either in your
home directory or at the root of this repository. They will execute every
time you start GDB. This is useful for automatically connecting to QEMU
and setting up your preferred debugging environment.

```
target remote tcp::1234
set architecture i386:x86-64
set disassembly-flavor intel
symbol-file ./bin/kernel.bin

display /i $pc

define longfix
    disconnect
    set architecture i386:x86-64:intel
    target remote tcp::1234
end
```

[GDB]: https://www.gnu.org/software/gdb/
[longfix]: http://wiki.osdev.org/QEMU_and_GDB_in_long_mode
