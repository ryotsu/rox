const path = require("path");
module.exports = {
  entry: "./index.ts",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "index.js",
  },
  mode: "development",
}
