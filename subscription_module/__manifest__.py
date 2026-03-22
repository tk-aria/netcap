{
    'name': 'サブスクリプション管理',
    'version': '18.0.1.0.0',
    'category': 'Sales',
    'summary': 'Subscription management for recurring revenue',
    'depends': ['sale', 'account'],
    'data': [
        'security/ir.model.access.csv',
        'data/cron.xml',
        'views/subscription_views.xml',
        'views/menu.xml',
    ],
    'installable': True,
    'license': 'LGPL-3',
}
