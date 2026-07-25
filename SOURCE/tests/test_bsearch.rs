fn main() {
    let mut arr: Vec<i128> = Vec::new();
    for i in 0..1000000 {
        arr.push(i as i128);
    }
    
    let target = 500000i128;
    let result = binary_search(&arr, target);
    
    match result {
        Some(idx) => println!("Found target at index {}", idx),
        None => println!("Target not found"),
    }
}

fn binary_search(arr: &[i128], target: i128) -> Option<usize> {
    let mut left = 0;
    let mut right = arr.len();
    
    while left < right {
        let mid = left + (right - left) / 2;
        
        if arr[mid] == target {
            return Some(mid);
        } else if arr[mid] < target {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    
    None
}
