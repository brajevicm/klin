const isNumber = require("is-number");

module.exports = { even: (n) => isNumber(n) && n % 2 === 0 };
