import "fmt";
import "strings";

// structs are plain data
struct Point {
    x: float;
    y: float;
}

// classes have fields and methods
class Animal {
    name: string;
    age: int;

    fn speak() -> string {
        return "...";
    }

    fn describe() -> string {
        return "I am " + self.name;
    }
}

class Dog {
    name: string;
    age: int;

    fn speak() -> string {
        return "Woof!";
    }

    fn fetch(item: string) -> string {
        return self.name + " fetched the " + item;
    }
}

// basic function
fn add(a: int, b: int) -> int {
    return a + b;
}

// optional return type
fn find(name: string) -> string? {
    if name == "mist" {
        return name;
    }
    return null;
}

// array types
fn sum(numbers: [int]) -> int {
    let total: int = 0;
    for n in numbers {
        total = total + n;
    }
    return total;
}

// multiple params, no return
fn greet(first: string, last: string) {
    let full: string = first + " " + last;
    print(full);
}

// structs initialized with fields
fn make_point(x: float, y: float) -> Point {
    return Point {
        x: x,
        y: y,
    };
}

// if / else
fn classify(n: int) -> string {
    if n < 0 {
        return "negative";
    } else {
        return "positive";
    }
}

// entry point
fn main() {
    // variables
    let x: int = 10;
    let y: int = 20;
    let result: int = add(x, y);

    // inferred (no type annotation)
    let message = "Hello from Mist!";
    print(message);

    // struct usage
    let p: Point = make_point(1.5, 2.5);
    print(p.x);

    // class usage
    let dog: Dog = Dog {
        name: "Rex",
        age: 3,
    };
    print(dog.speak());
    print(dog.fetch("ball"));

    // arrays
    let nums: [int] = [1, 2, 3, 4, 5];
    let total: int = sum(nums);
    print(total);

    // for loop
    for n in nums {
        print(n);
    }

    // optional
    let found: string? = find("mist");
    if found {
        print(found);
    }
}
