Denuos
======

> denuo [adverb]:
> new, over again, from a fresh beginning

**Denuos** is a project to satisfy all my [NIH] desires. I like to learn by
doing and thinking out problems for myself. I am endlessly fascinated by
the inner workings of modern computers. Through operating system design I plan
to explore many such subjects. Denuos is my personal learning experiment.

[NIH]: https://en.wikipedia.org/wiki/Not_invented_here

Usage
=====

Environment
-----------

Only a linux build environment is supported. Many of the tools necessary to
create a working Denuos image are quite difficult to get running on other
systems. Ubuntu is highly recommeneded. If you are running Windows or Mac OSX,
consider using [Docker] to provide a Linux environment.

```
docker run -it ubuntu bash
```

[Docker]: http://www.docker.com/products/docker

Dependencies
------------
```
sudo apt-get update
sudo apt-get install grub-common grub-pc-bin nasm qemu xorriso
```

Building and Running
--------------------
```
make iso
make run
```

Credits
=======
Denuos is inspired by several similar Rust projects. I would highly recommend
checking out the following projects:

- [Philipp Oppermann's Blog Series][phil]
- [intermezzOS][mzos]
- [Redox][redox]

[phil]: http://os.phil-opp.com/
[mzos]: https://intermezzos.github.io/
[redox]: https://www.redox-os.org/

License
=======

Copyright the Denuos Project Contributors.

Denuos is dual-licensed under the terms of the Apache License (Version 2.0)
or the MIT License at your option. For details, see
[LICENSE-APACHE](LICENSE-APACHE) or [LICENSE-MIT](LICENSE-MIT).

