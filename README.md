# argon

Fast greppable JSON!

argon is a command line tool heavily inspired by
[gron](https://github.com/tomnomnom/gron), but intending to be significantly
faster. The main mode of operation takes JSON and transforms it into a sequence
of assignment statements:
```
$ argon "http://date.jsontest.com"
json = {};
json.date = "07-02-2023";
json.milliseconds_since_epoch = 1688306513718;
json.time = "02:01:53 PM";
```

The inverse operation is also implemented:
```
$ argon "http://date.jsontest.com" | rg milli | argon --ungron
{
  "milliseconds_since_epoch": 1688306875807
}
```

argon differs from gron in being significantly faster. On
[this](https://github.com/json-iterator/test-data/blob/master/large-file.json)
25MB JSON file, I get the following time and memory usage on an old laptop:
```
$ make compare
..
gron:
real 4.77s
user 5.30s
sys  1.20s
maxmem 523764KB

argon:
real 0.39s
user 0.23s
sys  0.16s
maxmem 148292KB

gron --ungron:
Command terminated by signal 15
real 11.08s
user 26.16s
sys  10.62s
maxmem 3354592KB

argon --ungron:
real 0.72s
user 1.12s
sys  1.04s
maxmem 704424KB
```

Here, argon does gronning 12x faster taking 3.5x less memory. Ungronning is at
least 15x faster and takes at least 4.7x less memory. Ungronning using gron
never actually completes, as the program is killed after Out-Of-Memory
([earlyoom](https://github.com/rfjakob/earlyoom)) on my 8GB RAM laptop.

## Why it is fast

* The JSON manipulation is built upon
    [simd-json](https://github.com/simd-lite/simd-json). argon requires AVX2
    support.

* String unescaping takes significant time when deserializing json but is
    actually not necessary if we will escape it again soon afterwards. Much of
    the string escaping performed by simd-json is therefore patched out.

* argon is carefully written and profiled with performance in mind.

## Known functional differences from gron

* The `foo.bar` syntax is used over the `foo["bar"]` syntax even when `bar` is
    an invalid Javascript identifier. I.e., argon can output
    `json.foo\nbar = 123;`.
* argon will not escape initially unescaped
    [C1 control codes](https://en.wikipedia.org/wiki/C0_and_C1_control_codes).

## License

argon is dual-licensed under Apache-2.0 or MIT.
