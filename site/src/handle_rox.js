export class Handler {
  constructor(source) {
    this._source = source;
    this._output = [];
    this._error = [];
    this._opcode = [];
    this._error_lines = [];
  }

  get source() {
    return this._source;
  }

  get output() {
    return this._output.join('\n');
  }

  set output(o) {
    this._output.push(o);
  }

  get error() {
    return this._error.join('\n');
  }

  set error(error) {
    this._error.push(error);
  }

  get opcode() {
    return this._opcode.join('\n');
  }

  set opcode(op) {
    this._opcode.push(op);
  }

  get error_lines() {
    return this._error_lines;
  }

  set error_lines(line) {
    this._error_lines.push(line);
  }
}
