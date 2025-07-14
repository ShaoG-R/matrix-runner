// This code will not compile if `feature_build_fail` is enabled,
// because `non_existent_function` does not exist.
// This is used to test the build failure scenario.
//
// 如果启用了 `feature_build_fail`，此代码将无法编译，
// 因为 `non_existent_function` 不存在。
// 这用于测试构建失败的场景。
#[cfg(feature = "feature_build_fail")]
fn this_will_fail_to_compile() {
    non_existent_function();
}

#[cfg(test)]
mod tests {
    // This test will only be included if `feature_a` is enabled.
    // It should always pass.
    //
    // 这个测试只有在 `feature_a` 启用时才会被包含。
    // 它应该总是通过。
    #[test]
    #[cfg(feature = "feature_a")]
    fn test_with_feature_a() {
        assert_eq!(2 + 2, 4);
    }

    // This test will only be included if `feature_test_fail` is enabled.
    // It is designed to always panic, to test the test failure scenario.
    //
    // 这个测试只有在 `feature_test_fail` 启用时才会被包含。
    // 它被设计为总是会 panic，用于测试测试失败的场景。
    #[test]
    #[cfg(feature = "feature_test_fail")]
    fn test_that_fails() {
        panic!("This test is designed to fail.");
    }

    // A default test that is always present and should always pass.
    // This helps verify the test runner works even with no specific features.
    //
    // 一个始终存在并且应该总是通过的默认测试。
    // 这有助于验证测试运行器在没有特定 feature 的情况下也能工作。
    #[test]
    fn default_test_passes() {
        assert!(true);
    }
} 