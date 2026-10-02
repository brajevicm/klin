const yaml = require("yaml");

module.exports = { rules: (text) => yaml.parse(text) };
