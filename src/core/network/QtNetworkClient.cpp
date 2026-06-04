#include "core/network/QtNetworkClient.hpp"

#include <memory>

#include <QEventLoop>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>
#include <QUrl>

namespace easyenglish::core::network {

QtNetworkClient::QtNetworkClient(int timeout_ms) noexcept : timeout_ms_(timeout_ms) {}

auto QtNetworkClient::get(const QString& url) const -> std::expected<QByteArray, NetworkError> {
    // QNetworkAccessManager must live on the calling thread. We create a fresh
    // one per call so that this client is trivially safe to invoke from any
    // thread; cost is negligible compared to network latency.
    QNetworkAccessManager nam;
    QUrl qurl(url);
    // Brace-init to avoid the most-vexing-parse: `QNetworkRequest req(QUrl(x))`
    // would be parsed as a function declaration named `req`.
    QNetworkRequest request{qurl};
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute,
                         QNetworkRequest::NoLessSafeRedirectPolicy);

    std::unique_ptr<QNetworkReply> reply(nam.get(request));
    if (reply == nullptr) {
        return std::unexpected(NetworkError::Offline);
    }

    QEventLoop loop;
    QTimer timer;
    timer.setSingleShot(true);
    QObject::connect(&timer, &QTimer::timeout, &loop, &QEventLoop::quit);
    QObject::connect(reply.get(), &QNetworkReply::finished, &loop, &QEventLoop::quit);
    timer.start(timeout_ms_);
    loop.exec();

    if (!timer.isActive()) {
        // Timer fired first → request did not complete in time.
        reply->abort();
        return std::unexpected(NetworkError::Timeout);
    }
    timer.stop();

    if (reply->error() != QNetworkReply::NoError) {
        switch (reply->error()) {
            case QNetworkReply::HostNotFoundError:
            case QNetworkReply::ConnectionRefusedError:
            case QNetworkReply::NetworkSessionFailedError:
                return std::unexpected(NetworkError::Offline);
            case QNetworkReply::TimeoutError:
                return std::unexpected(NetworkError::Timeout);
            default:
                return std::unexpected(NetworkError::HttpError);
        }
    }

    const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
    if (status < 200 || status >= 300) {
        return std::unexpected(NetworkError::HttpError);
    }

    return reply->readAll();
}

}  // namespace easyenglish::core::network
