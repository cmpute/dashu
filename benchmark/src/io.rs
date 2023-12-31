use crate::number::{Float, Natural, Rational};
use std::{fmt::Debug, str::FromStr};

pub(crate) fn calculate_natural<T: Natural>(mut n: u32) -> String
where
    <T as FromStr>::Err: Debug,
{
    // generate input
    let digits = "1234567890";
    let mut input = String::with_capacity(n as usize);

    while n >= 10 {
        input.push_str(digits);
        n -= 10;
    }
    digits.chars().take(n as usize).for_each(|c| input.push(c));

    // parse, add one, then print
    let number = T::from_str(&input).unwrap();
    (number + T::from(1)).to_string()
}

pub(crate) fn calculate_ratioal<T: Rational>(mut n: u32) -> String
where
    <T as FromStr>::Err: Debug,
{
    // generate input
    let middle = n as usize / 2;
    let digits = "1234567890";
    let mut input = String::with_capacity(n as usize);

    while n >= 10 {
        input.push_str(digits);
        n -= 10;
    }
    digits.chars().take(n as usize).for_each(|c| input.push(c));
    input.insert(middle, '/');

    // parse, add one, then print
    let number = T::from_str(&input).unwrap();
    (number + T::from_u32(1)).to_string()
}

pub(crate) fn calculate_decimal<T: Float + FromStr>(mut n: u32) -> String
where
    <T as FromStr>::Err: Debug,
{
    // generate input
    let middle = n as usize / 2;
    let digits = "123456789";
    let mut input = String::with_capacity(n as usize);

    while n >= 10 {
        input.push_str(digits);
        n -= 10;
    }
    digits.chars().take(n as usize).for_each(|c| input.push(c));
    input.insert(middle, '.');

    // parse, add one, then print
    let number = T::from_str(&input).unwrap();
    (number + T::from(1)).to_string()
}
