package main;

import "fmt";

function main() {
    doSomething();
    let a = doSomethingElse();

    if (a + 1 == 67 + 1) {
        fmt.println(a);
    }
}

function doSomething() {
    fmt.println("hello");
}

function doSomethingElse(): int {
    fmt.println("world");

    return 67;
}
