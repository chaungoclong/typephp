<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Routing;

require_once __DIR__ . '/../Http/Message/ServerRequestInterface.php';
require_once __DIR__ . '/../Http/Message/ResponseInterface.php';
require_once __DIR__ . '/../Http/Message/Response.php';
require_once __DIR__ . '/../Http/Message/StringStream.php';

use TypePhp\MicroFw\Http\Message\ServerRequestInterface;
use TypePhp\MicroFw\Http\Message\ResponseInterface;
use TypePhp\MicroFw\Http\Message\Response;
use TypePhp\MicroFw\Http\Message\StringStream;

class App
{
    /** @var list<array{method: string, path: string, handler: \Closure(ServerRequestInterface): ResponseInterface}> */
    private array $routes;
    /** @var list<\Closure(ServerRequestInterface, \Closure(ServerRequestInterface): ResponseInterface): ResponseInterface> */
    private array $middlewares;

    public function __construct()
    {
        $this->routes = [];
        $this->middlewares = [];
    }

    /**
     * @param \Closure(ServerRequestInterface, \Closure(ServerRequestInterface): ResponseInterface): ResponseInterface $mw
     */
    public function addMiddleware(\Closure $mw): void
    {
        array_push($this->middlewares, $mw);
    }

    /**
     * @param \Closure(ServerRequestInterface): ResponseInterface $handler
     */
    public function get(string $path, \Closure $handler): void
    {
        /** @var array{method: string, path: string, handler: \Closure(ServerRequestInterface): ResponseInterface} $route */
        $route = ['method' => 'GET', 'path' => $path, 'handler' => $handler];
        array_push($this->routes, $route);
    }

    /**
     * @param \Closure(ServerRequestInterface): ResponseInterface $handler
     */
    public function post(string $path, \Closure $handler): void
    {
        /** @var array{method: string, path: string, handler: \Closure(ServerRequestInterface): ResponseInterface} $route */
        $route = ['method' => 'POST', 'path' => $path, 'handler' => $handler];
        array_push($this->routes, $route);
    }

    /**
     * @param \Closure(ServerRequestInterface): ResponseInterface $handler
     */
    public function put(string $path, \Closure $handler): void
    {
        /** @var array{method: string, path: string, handler: \Closure(ServerRequestInterface): ResponseInterface} $route */
        $route = ['method' => 'PUT', 'path' => $path, 'handler' => $handler];
        array_push($this->routes, $route);
    }

    /**
     * @param \Closure(ServerRequestInterface): ResponseInterface $handler
     */
    public function delete(string $path, \Closure $handler): void
    {
        /** @var array{method: string, path: string, handler: \Closure(ServerRequestInterface): ResponseInterface} $route */
        $route = ['method' => 'DELETE', 'path' => $path, 'handler' => $handler];
        array_push($this->routes, $route);
    }

    public function handle(ServerRequestInterface $req): ResponseInterface
    {
        /** @var \Closure(ServerRequestInterface): ResponseInterface $coreHandler */
        $coreHandler = fn (ServerRequestInterface $r): ResponseInterface => $this->dispatch($r);
        /** @var \Closure(ServerRequestInterface): ResponseInterface $pipeline */
        $pipeline = $coreHandler;
        foreach ($this->middlewares as $mw) {
            /** @var \Closure(ServerRequestInterface): ResponseInterface $next */
            $next = $pipeline;
            /** @var \Closure(ServerRequestInterface, \Closure(ServerRequestInterface): ResponseInterface): ResponseInterface $currentMw */
            $currentMw = $mw;
            $pipeline = fn (ServerRequestInterface $r): ResponseInterface => $currentMw($r, $next);
        }
        return $pipeline($req);
    }

    private function dispatch(ServerRequestInterface $req): ResponseInterface
    {
        $method = $req->getMethod();
        $path = $req->getUri()->getPath();
        foreach ($this->routes as $route) {
            if ($route['method'] === $method && $route['path'] === $path) {
                /** @var \Closure(ServerRequestInterface): ResponseInterface $h */
                $h = $route['handler'];
                return $h($req);
            }
        }
        $stream = new StringStream((string) json_encode(['ok' => false, 'error' => 'Route not found']));
        return new Response(404, ['content-type' => 'application/json; charset=utf-8'], $stream);
    }

    public function emit(ResponseInterface $res): void
    {
        http_response_code($res->getStatusCode());
        foreach ($res->getHeaders() as $name => $val) {
            header($name . ': ' . $val);
        }
        echo $res->getBody()->getContents();
    }
}
