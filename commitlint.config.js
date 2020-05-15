module.exports = {
    extends: ["@commitlint/config-conventional"],
    rules: {
        "header-min-length": [2, "always", 8],
        "header-max-length": [2, "always", 72],
        "subject-min-length": [2, "always", 3],
        "subject-max-length": [2, "always", 50],
        "body-max-line-length": [2, "always", 80]
    }
};
