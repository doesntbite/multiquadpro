import { handleBalancer } from './balancer.js';

export default {
  async fetch(request, env, ctx) {
    return await handleBalancer(env, request);
  }
};
