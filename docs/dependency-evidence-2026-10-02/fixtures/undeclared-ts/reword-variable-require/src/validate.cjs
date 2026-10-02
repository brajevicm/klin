const name = "csv-row-guard";
const { guard } = require(name);

module.exports = { validate: (text) => guard(text).ok };
