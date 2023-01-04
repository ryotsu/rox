export class Handler {
  constructor(source) {
    this._source = source;
    this._output = [];
    this._error = [];
    this._opcode = [];
    this._has_error = false;
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

  set error(e) {
    this._error.push(e);
  }

  get opcode() {
    return this._opcode.join('\n');
  }

  set opcode(op) {
    this._opcode.push(op);
  }

  set has_error(error) {
    this._has_error = error;
  }

  get has_error() {
    return this._has_error;
  }
}
