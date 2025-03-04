use std::time::Instant;

use expect_test::{expect_file, ExpectFile};
use ide_db::SymbolKind;
use test_utils::{bench, bench_fixture, skip_slow_tests, AssertLinear};

use crate::{fixture, FileRange, HlTag, TextRange};

#[test]
fn test_highlighting() {
    check_highlighting(
        r#"
//- proc_macros: identity, mirror
//- /main.rs crate:main deps:foo
use inner::{self as inner_mod};
mod inner {}

#[rustc_builtin_macro]
macro Copy {}

// Needed for function consuming vs normal
pub mod marker {
    #[lang = "copy"]
    pub trait Copy {}
}

#[proc_macros::identity]
pub mod ops {
    #[lang = "fn_once"]
    pub trait FnOnce<Args> {}

    #[lang = "fn_mut"]
    pub trait FnMut<Args>: FnOnce<Args> {}

    #[lang = "fn"]
    pub trait Fn<Args>: FnMut<Args> {}
}

proc_macros::mirror! {
    {
        ,i32 :x pub
        ,i32 :y pub
    } Foo struct
}

trait Bar where Self: {
    fn bar(&self) -> i32;
}

impl Bar for Foo where Self: {
    fn bar(&self) -> i32 {
        self.x
    }
}

impl Foo {
    fn baz(mut self, f: Foo) -> i32 {
        f.baz(self)
    }

    fn qux(&mut self) {
        self.x = 0;
    }

    fn quop(&self) -> i32 {
        self.x
    }
}

#[derive(Copy)]
struct FooCopy {
    x: u32,
}

impl FooCopy {
    fn baz(self, f: FooCopy) -> u32 {
        f.baz(self)
    }

    fn qux(&mut self) {
        self.x = 0;
    }

    fn quop(&self) -> u32 {
        self.x
    }
}

fn str() {
    str();
}

fn foo<'a, T>() -> T {
    foo::<'a, i32>()
}

fn never() -> ! {
    loop {}
}

fn const_param<const FOO: usize>() -> usize {
    FOO
}

use ops::Fn;
fn baz<F: Fn() -> ()>(f: F) {
    f()
}

fn foobar() -> impl Copy {}

fn foo() {
    let bar = foobar();
}

macro_rules! def_fn {
    ($($tt:tt)*) => {$($tt)*}
}

def_fn! {
    fn bar() -> u32 {
        100
    }
}

macro_rules! dont_color_me_braces {
    () => {0}
}

macro_rules! noop {
    ($expr:expr) => {
        $expr
    }
}

macro_rules! keyword_frag {
    ($type:ty) => ($type)
}

macro with_args($i:ident) {
    $i
}

macro without_args {
    ($i:ident) => {
        $i
    }
}

// comment
fn main() {
    println!("Hello, {}!", 92);
    dont_color_me_braces!();

    let mut vec = Vec::new();
    if true {
        let x = 92;
        vec.push(Foo { x, y: 1 });
    }

    for e in vec {
        // Do nothing
    }

    noop!(noop!(1));

    let mut x = 42;
    x += 1;
    let y = &mut x;
    let z = &y;

    let Foo { x: z, y } = Foo { x: z, y };

    y;

    let mut foo = Foo { x, y: x };
    let foo2 = Foo { x, y: x };
    foo.quop();
    foo.qux();
    foo.baz(foo2);

    let mut copy = FooCopy { x };
    copy.quop();
    copy.qux();
    copy.baz(copy);

    let a = |x| x;
    let bar = Foo::baz;

    let baz = (-42,);
    let baz = -baz.0;

    let _ = !true;

    'foo: loop {
        break 'foo;
        continue 'foo;
    }
}

enum Option<T> {
    Some(T),
    None,
}
use Option::*;

impl<T> Option<T> {
    fn and<U>(self, other: Option<U>) -> Option<(T, U)> {
        match other {
            None => unimplemented!(),
            Nope => Nope,
        }
    }
}

async fn learn_and_sing() {
    let song = learn_song().await;
    sing_song(song).await;
}

async fn async_main() {
    let f1 = learn_and_sing();
    let f2 = dance();
    futures::join!(f1, f2);
}

fn use_foo_items() {
    let bob = foo::Person {
        name: "Bob",
        age: foo::consts::NUMBER,
    };

    let control_flow = foo::identity(foo::ControlFlow::Continue);

    if control_flow.should_die() {
        foo::die!();
    }
}

pub enum Bool { True, False }

impl Bool {
    pub const fn to_primitive(self) -> bool {
        matches!(self, Self::True)
    }
}
const USAGE_OF_BOOL:bool = Bool::True.to_primitive();

//- /foo.rs crate:foo
pub struct Person {
    pub name: &'static str,
    pub age: u8,
}

pub enum ControlFlow {
    Continue,
    Die,
}

impl ControlFlow {
    pub fn should_die(self) -> bool {
        matches!(self, ControlFlow::Die)
    }
}

pub fn identity<T>(x: T) -> T { x }

pub mod consts {
    pub const NUMBER: i64 = 92;
}

macro_rules! die {
    () => {
        panic!();
    };
}
"#
        .trim(),
        expect_file!["./test_data/highlighting.html"],
        false,
    );
}

#[test]
fn test_rainbow_highlighting() {
    check_highlighting(
        r#"
fn main() {
    let hello = "hello";
    let x = hello.to_string();
    let y = hello.to_string();

    let x = "other color please!";
    let y = x.to_string();
}

fn bar() {
    let mut hello = "hello";
}
"#
        .trim(),
        expect_file!["./test_data/rainbow_highlighting.html"],
        true,
    );
}

#[test]
fn benchmark_syntax_highlighting_long_struct() {
    if skip_slow_tests() {
        return;
    }

    let fixture = bench_fixture::big_struct();
    let (analysis, file_id) = fixture::file(&fixture);

    let hash = {
        let _pt = bench("syntax highlighting long struct");
        analysis
            .highlight(file_id)
            .unwrap()
            .iter()
            .filter(|it| it.highlight.tag == HlTag::Symbol(SymbolKind::Struct))
            .count()
    };
    assert_eq!(hash, 2001);
}

#[test]
fn syntax_highlighting_not_quadratic() {
    if skip_slow_tests() {
        return;
    }

    let mut al = AssertLinear::default();
    while al.next_round() {
        for i in 6..=10 {
            let n = 1 << i;

            let fixture = bench_fixture::big_struct_n(n);
            let (analysis, file_id) = fixture::file(&fixture);

            let time = Instant::now();

            let hash = analysis
                .highlight(file_id)
                .unwrap()
                .iter()
                .filter(|it| it.highlight.tag == HlTag::Symbol(SymbolKind::Struct))
                .count();
            assert!(hash > n as usize);

            let elapsed = time.elapsed();
            al.sample(n as f64, elapsed.as_millis() as f64);
        }
    }
}

#[test]
fn benchmark_syntax_highlighting_parser() {
    if skip_slow_tests() {
        return;
    }

    let fixture = bench_fixture::glorious_old_parser();
    let (analysis, file_id) = fixture::file(&fixture);

    let hash = {
        let _pt = bench("syntax highlighting parser");
        analysis
            .highlight(file_id)
            .unwrap()
            .iter()
            .filter(|it| it.highlight.tag == HlTag::Symbol(SymbolKind::Function))
            .count()
    };
    assert_eq!(hash, 1616);
}

#[test]
fn test_ranges() {
    let (analysis, file_id) = fixture::file(
        r#"
#[derive(Clone, Debug)]
struct Foo {
    pub x: i32,
    pub y: i32,
}
"#,
    );

    // The "x"
    let highlights = &analysis
        .highlight_range(FileRange { file_id, range: TextRange::at(45.into(), 1.into()) })
        .unwrap();

    assert_eq!(&highlights[0].highlight.to_string(), "field.declaration.public");
}

#[test]
fn test_flattening() {
    check_highlighting(
        r##"
fn fixture(ra_fixture: &str) {}

fn main() {
    fixture(r#"
        trait Foo {
            fn foo() {
                println!("2 + 2 = {}", 4);
            }
        }"#
    );
}"##
        .trim(),
        expect_file!["./test_data/highlight_injection.html"],
        false,
    );
}

#[test]
fn ranges_sorted() {
    let (analysis, file_id) = fixture::file(
        r#"
#[foo(bar = "bar")]
macro_rules! test {}
}"#
        .trim(),
    );
    let _ = analysis.highlight(file_id).unwrap();
}

#[test]
fn test_string_highlighting() {
    // The format string detection is based on macro-expansion,
    // thus, we have to copy the macro definition from `std`
    check_highlighting(
        r#"
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::io::_print($crate::format_args_nl!($($arg)*));
    })
}
#[rustc_builtin_macro]
macro_rules! format_args_nl {
    ($fmt:expr) => {{ /* compiler built-in */ }};
    ($fmt:expr, $($args:tt)*) => {{ /* compiler built-in */ }};
}

fn main() {
    // from https://doc.rust-lang.org/std/fmt/index.html
    println!("Hello");                 // => "Hello"
    println!("Hello, {}!", "world");   // => "Hello, world!"
    println!("The number is {}", 1);   // => "The number is 1"
    println!("{:?}", (3, 4));          // => "(3, 4)"
    println!("{value}", value=4);      // => "4"
    println!("{} {}", 1, 2);           // => "1 2"
    println!("{:04}", 42);             // => "0042" with leading zerosV
    println!("{1} {} {0} {}", 1, 2);   // => "2 1 1 2"
    println!("{argument}", argument = "test");   // => "test"
    println!("{name} {}", 1, name = 2);          // => "2 1"
    println!("{a} {c} {b}", a="a", b='b', c=3);  // => "a 3 b"
    println!("{{{}}}", 2);                       // => "{2}"
    println!("Hello {:5}!", "x");
    println!("Hello {:1$}!", "x", 5);
    println!("Hello {1:0$}!", 5, "x");
    println!("Hello {:width$}!", "x", width = 5);
    println!("Hello {:<5}!", "x");
    println!("Hello {:-<5}!", "x");
    println!("Hello {:^5}!", "x");
    println!("Hello {:>5}!", "x");
    println!("Hello {:+}!", 5);
    println!("{:#x}!", 27);
    println!("Hello {:05}!", 5);
    println!("Hello {:05}!", -5);
    println!("{:#010x}!", 27);
    println!("Hello {0} is {1:.5}", "x", 0.01);
    println!("Hello {1} is {2:.0$}", 5, "x", 0.01);
    println!("Hello {0} is {2:.1$}", "x", 5, 0.01);
    println!("Hello {} is {:.*}",    "x", 5, 0.01);
    println!("Hello {} is {2:.*}",   "x", 5, 0.01);
    println!("Hello {} is {number:.prec$}", "x", prec = 5, number = 0.01);
    println!("{}, `{name:.*}` has 3 fractional digits", "Hello", 3, name=1234.56);
    println!("{}, `{name:.*}` has 3 characters", "Hello", 3, name="1234.56");
    println!("{}, `{name:>8.*}` has 3 right-aligned characters", "Hello", 3, name="1234.56");
    println!("Hello {{}}");
    println!("{{ Hello");

    println!(r"Hello, {}!", "world");

    // escape sequences
    println!("Hello\nWorld");
    println!("\u{48}\x65\x6C\x6C\x6F World");

    println!("{\x41}", A = 92);
    println!("{ничоси}", ничоси = 92);

    println!("{:x?} {} ", thingy, n2);
}"#
        .trim(),
        expect_file!["./test_data/highlight_strings.html"],
        false,
    );
}

#[test]
fn test_unsafe_highlighting() {
    check_highlighting(
        r#"
static mut MUT_GLOBAL: Struct = Struct { field: 0 };
static GLOBAL: Struct = Struct { field: 0 };
unsafe fn unsafe_fn() {}

union Union {
    a: u32,
    b: f32,
}

struct Struct { field: i32 }
impl Struct {
    unsafe fn unsafe_method(&self) {}
}

#[repr(packed)]
struct Packed {
    a: u16,
}

unsafe trait UnsafeTrait {}
unsafe impl UnsafeTrait for Packed {}
impl !UnsafeTrait for () {}

fn unsafe_trait_bound<T: UnsafeTrait>(_: T) {}

trait DoTheAutoref {
    fn calls_autoref(&self);
}

impl DoTheAutoref for u16 {
    fn calls_autoref(&self) {}
}

fn main() {
    let x = &5 as *const _ as *const usize;
    let u = Union { b: 0 };
    unsafe {
        // unsafe fn and method calls
        unsafe_fn();
        let b = u.b;
        match u {
            Union { b: 0 } => (),
            Union { a } => (),
        }
        Struct { field: 0 }.unsafe_method();

        // unsafe deref
        *x;

        // unsafe access to a static mut
        MUT_GLOBAL.field;
        GLOBAL.field;

        // unsafe ref of packed fields
        let packed = Packed { a: 0 };
        let a = &packed.a;
        let ref a = packed.a;
        let Packed { ref a } = packed;
        let Packed { a: ref _a } = packed;

        // unsafe auto ref of packed field
        packed.a.calls_autoref();
    }
}
"#
        .trim(),
        expect_file!["./test_data/highlight_unsafe.html"],
        false,
    );
}

#[test]
fn test_highlight_doc_comment() {
    check_highlighting(
        r#"
//! This is a module to test doc injection.
//! ```
//! fn test() {}
//! ```

/// ```
/// let _ = "early doctests should not go boom";
/// ```
struct Foo {
    bar: bool,
}

/// This is an impl with a code block.
///
/// ```
/// fn foo() {
///
/// }
/// ```
impl Foo {
    /// ```
    /// let _ = "Call me
    //    KILLER WHALE
    ///     Ishmael.";
    /// ```
    pub const bar: bool = true;

    /// Constructs a new `Foo`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(unused_mut)]
    /// let mut foo: Foo = Foo::new();
    /// ```
    pub const fn new() -> Foo {
        Foo { bar: true }
    }

    /// `bar` method on `Foo`.
    ///
    /// # Examples
    ///
    /// ```
    /// use x::y;
    ///
    /// let foo = Foo::new();
    ///
    /// // calls bar on foo
    /// assert!(foo.bar());
    ///
    /// let bar = foo.bar || Foo::bar;
    ///
    /// /* multi-line
    ///        comment */
    ///
    /// let multi_line_string = "Foo
    ///   bar\n
    ///          ";
    ///
    /// ```
    ///
    /// ```rust,no_run
    /// let foobar = Foo::new().bar();
    /// ```
    ///
    /// ```sh
    /// echo 1
    /// ```
    pub fn foo(&self) -> bool {
        true
    }
}

/// [`Foo`](Foo) is a struct
/// This function is > [`all_the_links`](all_the_links) <
/// [`noop`](noop) is a macro below
/// [`Item`] is a struct in the module [`module`]
///
/// [`Item`]: module::Item
/// [mix_and_match]: ThisShouldntResolve
pub fn all_the_links() {}

pub mod module {
    pub struct Item;
}

/// ```
/// noop!(1);
/// ```
macro_rules! noop {
    ($expr:expr) => {
        $expr
    }
}

/// ```rust
/// let _ = example(&[1, 2, 3]);
/// ```
///
/// ```
/// loop {}
#[cfg_attr(not(feature = "false"), doc = "loop {}")]
#[doc = "loop {}"]
/// ```
///
#[cfg_attr(feature = "alloc", doc = "```rust")]
#[cfg_attr(not(feature = "alloc"), doc = "```ignore")]
/// let _ = example(&alloc::vec![1, 2, 3]);
/// ```
pub fn mix_and_match() {}

/**
It is beyond me why you'd use these when you got ///
```rust
let _ = example(&[1, 2, 3]);
```
[`block_comments2`] tests these with indentation
 */
pub fn block_comments() {}

/**
    Really, I don't get it
    ```rust
    let _ = example(&[1, 2, 3]);
    ```
    [`block_comments`] tests these without indentation
*/
pub fn block_comments2() {}
"#
        .trim(),
        expect_file!["./test_data/highlight_doctest.html"],
        false,
    );
}

#[test]
fn test_extern_crate() {
    check_highlighting(
        r#"
        //- /main.rs crate:main deps:std,alloc
        extern crate std;
        extern crate alloc as abc;
        //- /std/lib.rs crate:std
        pub struct S;
        //- /alloc/lib.rs crate:alloc
        pub struct A
        "#,
        expect_file!["./test_data/highlight_extern_crate.html"],
        false,
    );
}

#[test]
fn test_associated_function() {
    check_highlighting(
        r#"
fn not_static() {}

struct foo {}

impl foo {
    pub fn is_static() {}
    pub fn is_not_static(&self) {}
}

trait t {
    fn t_is_static() {}
    fn t_is_not_static(&self) {}
}

impl t for foo {
    pub fn is_static() {}
    pub fn is_not_static(&self) {}
}
        "#,
        expect_file!["./test_data/highlight_assoc_functions.html"],
        false,
    )
}

#[test]
fn test_injection() {
    check_highlighting(
        r##"
fn f(ra_fixture: &str) {}
fn main() {
    f(r"
fn foo() {
    foo(\$0{
        92
    }\$0)
}");
}
    "##,
        expect_file!["./test_data/injection.html"],
        false,
    );
}

/// Highlights the code given by the `ra_fixture` argument, renders the
/// result as HTML, and compares it with the HTML file given as `snapshot`.
/// Note that the `snapshot` file is overwritten by the rendered HTML.
fn check_highlighting(ra_fixture: &str, expect: ExpectFile, rainbow: bool) {
    let (analysis, file_id) = fixture::file(ra_fixture);
    let actual_html = &analysis.highlight_as_html(file_id, rainbow).unwrap();
    expect.assert_eq(actual_html)
}
