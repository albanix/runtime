You can:
```rs
async fn w() -> i8 {
    123
}

fn main() {
    executor::spawn(|| { 
        w().await
    });
}
```