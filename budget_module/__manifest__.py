{
    'name': '予算管理',
    'version': '18.0.1.0.0',
    'category': 'Accounting',
    'summary': 'Budget Management for Community Edition',
    'depends': ['account'],
    'data': [
        'security/ir.model.access.csv',
        'views/budget_views.xml',
        'views/menu.xml',
    ],
    'installable': True,
    'application': False,
    'license': 'LGPL-3',
}
