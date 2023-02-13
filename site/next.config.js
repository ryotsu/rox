module.exports = {
  webpack: function (config, _options) {
    config.experiments = { asyncWebAssembly: true, syncWebAssembly: true };
    return config;
  },
}
