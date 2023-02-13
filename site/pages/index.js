import { greet } from '../node_modules/rox/rox_bg';

function Home() {
  return <h1>{greet("World")}</h1>;
}

export default Home;
