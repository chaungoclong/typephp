<?php

declare(strict_types=1);

require_once __DIR__ . '/../src/Http/Message/ServerRequestInterface.php';
require_once __DIR__ . '/../src/Http/Message/ResponseInterface.php';
require_once __DIR__ . '/../src/Http/Message/ServerRequest.php';
require_once __DIR__ . '/../src/Http/Message/Response.php';
require_once __DIR__ . '/../src/Http/Message/StringStream.php';
require_once __DIR__ . '/../src/Http/Message/Uri.php';
require_once __DIR__ . '/../src/Routing/App.php';

use TypePhp\MicroFw\Http\Message\ServerRequestInterface;
use TypePhp\MicroFw\Http\Message\ResponseInterface;
use TypePhp\MicroFw\Http\Message\ServerRequest;
use TypePhp\MicroFw\Http\Message\Response;
use TypePhp\MicroFw\Http\Message\StringStream;
use TypePhp\MicroFw\Http\Message\Uri;
use TypePhp\MicroFw\Routing\App;

function jsonResponse(int $status, mixed $data): ResponseInterface
{
    $stream = new StringStream((string) json_encode($data));
    return new Response($status, ['content-type' => 'application/json; charset=utf-8'], $stream);
}

function createServerRequestFromGlobals(): ServerRequestInterface
{
    $method = 'GET';
    $rawMethod = $_SERVER['REQUEST_METHOD'] ?? 'GET';
    if (is_string($rawMethod)) {
        $method = strtoupper($rawMethod);
    }
    $rawUri = '/';
    $serverUri = $_SERVER['REQUEST_URI'] ?? '/';
    if (is_string($serverUri)) {
        $rawUri = $serverUri;
    }
    $path = '/';
    $parsedPath = parse_url($rawUri, PHP_URL_PATH);
    if (is_string($parsedPath)) {
        $path = $parsedPath;
    }
    $host = 'localhost';
    $rawHost = $_SERVER['HTTP_HOST'] ?? 'localhost';
    if (is_string($rawHost)) {
        $host = $rawHost;
    }
    $rawBody = (string) file_get_contents('php://input');
    $parsed = null;
    if (trim($rawBody) !== '') {
        $parsed = json_decode($rawBody, true);
    }
    $uri = new Uri('http', $host, $path);
    $stream = new StringStream($rawBody);
    return new ServerRequest($method, $uri, [], $stream, $_GET, $parsed, []);
}

$app = new App();
$app->addMiddleware(fn (ServerRequestInterface $req, \Closure $next): ResponseInterface => $next($req->withAttribute('framework', 'TypePHP-PSR7')));
$app->get('/', fn (ServerRequestInterface $req): ResponseInterface => jsonResponse(200, ['ok' => true, 'framework' => $req->getAttribute('framework'), 'path' => $req->getUri()->getPath()]));
$app->get('/hello', fn (ServerRequestInterface $req): ResponseInterface => jsonResponse(200, ['ok' => true, 'message' => 'Hello from PSR-4 Composer Package!', 'query' => $req->getQueryParams()]));
$app->post('/echo', fn (ServerRequestInterface $req): ResponseInterface => jsonResponse(200, ['ok' => true, 'received' => $req->getParsedBody()]));
$request = createServerRequestFromGlobals();
$response = $app->handle($request);
$app->emit($response);
