# LC3 Virtual Machine
This is an implementation of a LC3 architecture virtual machine in rust based on [Justin Meiners tutorial](https://www.jmeiners.com/lc3-vm/). 

## What is LC3 architecture?
LC3 as in Little Computer 3, is an educational computer architecture with a simplified instruction set compared to x86, but that still demonstrates the main ideas used by modern CPUs.

## What is a Virtual Machine?
A Virtual Machine (VM) is a program that acts like a computer simulating a CPU along with a few other hardware components. In short, VMs can perform arithmetic operations, read and write to memory, and interact with I/O devices, just like physical computers, and most importantly, it can understand a machine language which you can use to program it.

## Quick Start
The project comes with two example targets programmed in LC3 assembly that you can run on the vm. This targets are two games: 2048 and rogue. To run them on the vm you can run either of the following commands respectivly:

```
make 2048
```

```
make rogue
```

To run the vm on any other image, run:

```
make run path=[target]
```

For example:

```
make run path=example_images/2048.obj
```

- To build the project, run:
```
make build
```
- To run the tests, run:
```
make test
```

## About this project
This is vm implementation consists of two modules: **lc3_vm** with all the execution logic and the console (i/o) and memory management, and **hardware** with all the hardware components. 

## Dependencies
- rust 1.85.0
- console 0.15.0
- raw_tty 0.1.0
- termios 0.3.0
- timeout-readwrite 0.4.0

## References
- https://www.jmeiners.com/lc3-vm/
- https://en.wikipedia.org/wiki/Little_Computer_3
