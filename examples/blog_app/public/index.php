<?php

declare(strict_types=1);
// Target: PHP 8.4

require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/ServerRequestInterface.php';
require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/ResponseInterface.php';
require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/ServerRequest.php';
require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/Response.php';
require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/StringStream.php';
require_once __DIR__ . '/../../psr7_microfw/src/Http/Message/Uri.php';
require_once __DIR__ . '/../../psr7_microfw/src/Routing/App.php';
require_once __DIR__ . '/../../psr7_microfw/src/Validation/Validator.php';
require_once __DIR__ . '/../../psr7_microfw/src/Validation/ValidationResult.php';
require_once __DIR__ . '/../src/Repository/PostRepository.php';
require_once __DIR__ . '/../src/View/Templates.php';

use TypePhp\MicroFw\Http\Message\ServerRequestInterface;
use TypePhp\MicroFw\Http\Message\ResponseInterface;
use TypePhp\MicroFw\Http\Message\ServerRequest;
use TypePhp\MicroFw\Http\Message\Response;
use TypePhp\MicroFw\Http\Message\StringStream;
use TypePhp\MicroFw\Http\Message\Uri;
use TypePhp\MicroFw\Routing\App;
use TypePhp\MicroFw\Validation\Validator;
use TypePhp\MicroFw\Validation\ValidationResult;
use TypePhp\BlogApp\Repository\PostRepository;
use TypePhp\BlogApp\View\Templates;

const ADMIN_USER = 'admin';
const ADMIN_PASS = 'admin123';
const SESSION_VALUE = 'typephp_admin_authenticated';
function jsonResponse(int $status, mixed $data): ResponseInterface
{
    $stream = new StringStream((string) json_encode($data));
    return new Response($status, ['content-type' => 'application/json; charset=utf-8'], $stream);
}

function htmlResponse(int $status, string $html): ResponseInterface
{
    $stream = new StringStream($html);
    return new Response($status, ['content-type' => 'text/html; charset=utf-8'], $stream);
}

function redirectResponse(string $location): ResponseInterface
{
    return new Response(302, ['location' => $location], new StringStream(''));
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
    /** @var array<string, string> $headers */
    $headers = [];
    $sessionCookie = $_COOKIE['blog_admin'] ?? null;
    if (is_string($sessionCookie)) {
        $headers['x-session'] = $sessionCookie;
    }
    $rawBody = (string) file_get_contents('php://input');
    $parsed = null;
    if (trim($rawBody) !== '') {
        $parsed = json_decode($rawBody, true);
    }
    $uri = new Uri('http', $host, $path);
    $stream = new StringStream($rawBody);
    return new ServerRequest($method, $uri, $headers, $stream, $_GET, $parsed, []);
}

function isAuthenticated(ServerRequestInterface $req): bool
{
    return $req->getHeaderLine('x-session') === SESSION_VALUE;
}

function getQueryInt(ServerRequestInterface $req, string $key): int
{
    /** @var array<string, mixed> $params */
    $params = $req->getQueryParams();
    if (!array_key_exists($key, $params)) {
        return 0;
    }
    $val = $params[$key];
    if (is_int($val)) {
        return $val;
    }
    if (is_string($val)) {
        return intval($val);
    }
    return 0;
}

/**
 * @param \Closure(ServerRequestInterface): ResponseInterface $next
 */
function authMiddleware(ServerRequestInterface $req, \Closure $next): ResponseInterface
{
    $path = $req->getUri()->getPath();
    if ($path === '/admin' && !isAuthenticated($req)) {
        return redirectResponse('/login');
    }
    if (str_starts_with($path, '/api/admin') && !isAuthenticated($req)) {
        return jsonResponse(401, ['ok' => false, 'error' => 'Chưa đăng nhập Admin']);
    }
    return $next($req);
}

function handleLogin(ServerRequestInterface $req, Validator $validator): ResponseInterface
{
    $res = $validator->validate($req->getParsedBody(), ['username' => 'required|string', 'password' => 'required|string']);
    if (!$res->isValid()) {
        return jsonResponse(422, ['ok' => false, 'error' => 'Vui lòng nhập đầy đủ tài khoản và mật khẩu']);
    }
    /** @var array<string, mixed> $data */
    $data = $res->getData();
    $user = '';
    $rawUser = $data['username'] ?? null;
    if (is_string($rawUser)) {
        $user = $rawUser;
    }
    $pass = '';
    $rawPass = $data['password'] ?? null;
    if (is_string($rawPass)) {
        $pass = $rawPass;
    }
    if ($user !== ADMIN_USER || $pass !== ADMIN_PASS) {
        return jsonResponse(401, ['ok' => false, 'error' => 'Sai tên đăng nhập hoặc mật khẩu']);
    }
    setcookie('blog_admin', SESSION_VALUE, 0, '/');
    return jsonResponse(200, ['ok' => true]);
}

function handleLogout(): ResponseInterface
{
    setcookie('blog_admin', '', 1, '/');
    return jsonResponse(200, ['ok' => true]);
}

function handleViewPostHtml(ServerRequestInterface $req, PostRepository $repo, Templates $views): ResponseInterface
{
    $id = getQueryInt($req, 'id');
    if ($id <= 0) {
        return htmlResponse(404, $views->renderPostDetailPage(null));
    }
    $post = $repo->findById($id);
    if (!is_array($post)) {
        return htmlResponse(404, $views->renderPostDetailPage(null));
    }
    $rawPub = $post['published'] ?? 0;
    $pubInt = 0;
    if (is_int($rawPub)) {
        $pubInt = $rawPub;
    } elseif (is_string($rawPub)) {
        $pubInt = intval($rawPub);
    }
    if ($pubInt !== 1) {
        return htmlResponse(404, $views->renderPostDetailPage(null));
    }
    return htmlResponse(200, $views->renderPostDetailPage($post));
}

function handleAdminCreatePost(ServerRequestInterface $req, PostRepository $repo, Validator $validator): ResponseInterface
{
    $res = $validator->validate($req->getParsedBody(), ['title' => 'required|string|min:3|max:150', 'slug' => 'required|string|min:3|max:100', 'content' => 'required|string|min:10', 'published' => 'bool']);
    if (!$res->isValid()) {
        return jsonResponse(422, ['ok' => false, 'errors' => $res->getErrors()]);
    }
    /** @var array<string, mixed> $data */
    $data = $res->getData();
    $title = '';
    $rawTitle = $data['title'] ?? null;
    if (is_string($rawTitle)) {
        $title = $rawTitle;
    }
    $slug = '';
    $rawSlug = $data['slug'] ?? null;
    if (is_string($rawSlug)) {
        $slug = $rawSlug;
    }
    $content = '';
    $rawContent = $data['content'] ?? null;
    if (is_string($rawContent)) {
        $content = $rawContent;
    }
    $published = 0;
    $rawPublished = $data['published'] ?? false;
    if ($rawPublished === true) {
        $published = 1;
    }
    $created = $repo->create($title, $slug, $content, $published);
    return jsonResponse(201, ['ok' => true, 'data' => $created]);
}

function handleAdminUpdatePost(ServerRequestInterface $req, PostRepository $repo, Validator $validator): ResponseInterface
{
    $id = getQueryInt($req, 'id');
    if ($id <= 0) {
        return jsonResponse(400, ['ok' => false, 'error' => 'Query parameter id is required']);
    }
    $existing = $repo->findById($id);
    if (!is_array($existing)) {
        return jsonResponse(404, ['ok' => false, 'error' => 'Post not found']);
    }
    $res = $validator->validate($req->getParsedBody(), ['title' => 'required|string|min:3|max:150', 'slug' => 'required|string|min:3|max:100', 'content' => 'required|string|min:10', 'published' => 'bool']);
    if (!$res->isValid()) {
        return jsonResponse(422, ['ok' => false, 'errors' => $res->getErrors()]);
    }
    /** @var array<string, mixed> $data */
    $data = $res->getData();
    $title = '';
    $rawTitle = $data['title'] ?? null;
    if (is_string($rawTitle)) {
        $title = $rawTitle;
    }
    $slug = '';
    $rawSlug = $data['slug'] ?? null;
    if (is_string($rawSlug)) {
        $slug = $rawSlug;
    }
    $content = '';
    $rawContent = $data['content'] ?? null;
    if (is_string($rawContent)) {
        $content = $rawContent;
    }
    $published = 0;
    $rawPublished = $data['published'] ?? false;
    if ($rawPublished === true) {
        $published = 1;
    }
    $repo->update($id, $title, $slug, $content, $published);
    return jsonResponse(200, ['ok' => true, 'data' => $repo->findById($id)]);
}

function handleAdminDeletePost(ServerRequestInterface $req, PostRepository $repo): ResponseInterface
{
    $id = getQueryInt($req, 'id');
    if ($id <= 0) {
        return jsonResponse(400, ['ok' => false, 'error' => 'Query parameter id is required']);
    }
    if (!$repo->delete($id)) {
        return jsonResponse(404, ['ok' => false, 'error' => 'Post not found']);
    }
    return jsonResponse(200, ['ok' => true, 'data' => 'Deleted']);
}

function handleLoginPage(ServerRequestInterface $req, Templates $views): ResponseInterface
{
    if (isAuthenticated($req)) {
        return redirectResponse('/admin');
    }
    return htmlResponse(200, $views->renderLoginPage());
}

$repo = new PostRepository(__DIR__ . '/blog.db');
$validator = new Validator();
$views = new Templates();
$app = new App();
$app->addMiddleware(fn (ServerRequestInterface $req, \Closure $next): ResponseInterface => authMiddleware($req, $next));
$app->get('/', fn (ServerRequestInterface $req): ResponseInterface => htmlResponse(200, $views->renderBlogPage($repo->findPublished())));
$app->get('/post', fn (ServerRequestInterface $req): ResponseInterface => handleViewPostHtml($req, $repo, $views));
$app->get('/login', fn (ServerRequestInterface $req): ResponseInterface => handleLoginPage($req, $views));
$app->get('/admin', fn (ServerRequestInterface $req): ResponseInterface => htmlResponse(200, $views->renderAdminPage()));
$app->post('/api/login', fn (ServerRequestInterface $req): ResponseInterface => handleLogin($req, $validator));
$app->post('/api/logout', fn (ServerRequestInterface $req): ResponseInterface => handleLogout());
$app->get('/api/posts', fn (ServerRequestInterface $req): ResponseInterface => jsonResponse(200, ['ok' => true, 'data' => $repo->findPublished()]));
$app->get('/api/admin/posts', fn (ServerRequestInterface $req): ResponseInterface => jsonResponse(200, ['ok' => true, 'data' => $repo->findAll()]));
$app->post('/api/admin/posts', fn (ServerRequestInterface $req): ResponseInterface => handleAdminCreatePost($req, $repo, $validator));
$app->put('/api/admin/posts/update', fn (ServerRequestInterface $req): ResponseInterface => handleAdminUpdatePost($req, $repo, $validator));
$app->delete('/api/admin/posts/delete', fn (ServerRequestInterface $req): ResponseInterface => handleAdminDeletePost($req, $repo));
$request = createServerRequestFromGlobals();
$response = $app->handle($request);
$app->emit($response);
