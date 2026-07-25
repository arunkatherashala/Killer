fn main() {
    let mut arr: Vec<i128> = Vec::new();
    for i in 0..1000 {
        arr.push((1000 - i) as i128);
    }
    
    arr = mergesort(arr);
    println!("Mergesorted {} elements. First: {}, Last: {}", arr.len(), arr[0], arr[arr.len()-1]);
}

fn mergesort(arr: Vec<i128>) -> Vec<i128> {
    if arr.len() <= 1 { return arr; }
    
    let mid = arr.len() / 2;
    let left = mergesort(arr[0..mid].to_vec());
    let right = mergesort(arr[mid..].to_vec());
    
    merge(left, right)
}

fn merge(left: Vec<i128>, right: Vec<i128>) -> Vec<i128> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;
    
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            result.push(left[i]);
            i += 1;
        } else {
            result.push(right[j]);
            j += 1;
        }
    }
    
    while i < left.len() { result.push(left[i]); i += 1; }
    while j < right.len() { result.push(right[j]); j += 1; }
    
    result
}
