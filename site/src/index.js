import * as monaco from 'monaco-editor';
import * as rox from '../node_modules/rox/rox.js';
import { Handler } from './handle_rox.js';

import './style.css';

let editor = monaco.editor.create(document.getElementById('source'), {
  value: ['print "Hello, World!";'].join('\n'),
  theme: 'vs-dark',
  glyphMargin: true,
});

let output = monaco.editor.create(document.getElementById('output'), {
  value: '',
  readOnly: true,
  lineNumbers: 'off',
});

let opcode = monaco.editor.create(document.getElementById('opcode'), {
  value: '',
  readOnly: true,
  lineNumbers: 'off',
});

document.getElementById('print').addEventListener('click', (_evt) => {
  let source = editor.getValue();
  let handler = new Handler(source);
  rox.run(handler);

  let result = handler.output + '\n' + handler.error;
  output.setValue(result.trim());

  opcode.setValue(handler.opcode);
});
