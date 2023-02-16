const { join } = require('node:path');
const { access, symlink } = require('node:fs/promises');

const repo = 'rox'
const assetPrefix = `/${repo}/`
const basePath = `/${repo}`


module.exports = {
  assetPrefix: assetPrefix,
  basePath: basePath,
  webpack: function (config, { isServer }) {
    config.experiments = { asyncWebAssembly: true, syncWebAssembly: true };
    config.plugins.push(
      new (class {
        apply(compiler) {
          compiler.hooks.afterEmit.tapPromise(
            'SymlinkWebpackPlugin',
            async (compiler) => {
              if (isServer) {
                const from = join(compiler.options.output.path, '../static');
                const to = join(compiler.options.output.path, 'static');

                try {
                  await access(from);
                  console.log(`${from} already exists`);
                  return;
                } catch (error) {
                  if (error.code === 'ENOENT') {
                    // No link exists
                  } else {
                    throw error;
                  }
                }

                await symlink(to, from, 'junction');
                console.log(`created symlink ${from} -> ${to}`);
              }
            },
          );
        }
      })(),
    );
    return config;
  },
}

